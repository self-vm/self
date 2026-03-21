use futures::future::BoxFuture;
use tokio::sync::mpsc;

use crate::core::error::struct_errors::StructError;
use crate::core::error::type_errors::TypeError;
use crate::core::error::vector_errors::VectorError;
use crate::core::error::InvalidBinaryOperation;
use crate::core::error::VMErrorType;
use crate::core::execution::VMExecutionResult;
use crate::core::handlers::call_handler::call_handler;
use crate::core::handlers::foreign_handlers::ForeignHandlers;
use crate::core::handlers::print_handler::print_handler;
use crate::events::Event;
use crate::memory::Handle;
use crate::memory::MemObject;
use crate::memory::MemoryManager;
use crate::std::bootstrap_default_lib;
use crate::std::builtin_functions;
use crate::std::heap_utils::put_string;
use crate::std::vector;
use crate::std::{generate_native_module, get_native_module_type};
use crate::types::object::func::Engine;
use crate::types::object::func::Function;
use crate::types::object::string::SelfString;
use crate::types::object::structs::StructDeclaration;
use crate::types::object::structs::StructLiteral;
use crate::types::object::vector::Vector;
use crate::types::object::BoundAccess;
use crate::types::raw::utf8::Utf8;
use crate::types::raw::RawValue;
use crate::types::raw::{bool::Bool, f64::F64, i32::I32, i64::I64, u32::U32, u64::U64};
use crate::utils::foreign_handlers_utils::get_foreign_handlers;
use self_bytecode::DataType;
use self_bytecode::Opcode;
use std::collections::HashMap;
use std::path::Path;

use super::stack::*;
use super::types::*;

pub struct Vm {
    operand_stack: Vec<OperandsStackValue>,
    pub call_stack: CallStack,
    pub memory: MemoryManager,
    bytecode: Vec<u8>,
    pc: usize,
    pub handlers: HashMap<String, Handle>,
    ffi_handlers: ForeignHandlers,
    events_queue: mpsc::UnboundedReceiver<Event>,
    events_sender: mpsc::UnboundedSender<Event>,
}

impl Vm {
    pub fn new(bytecode: Vec<u8>) -> Vm {
        //let mut translator = Translator::new(bytecode);
        //let instructions = translator.translate();

        // load ffi_handlers
        let mut ffi_handlers = ForeignHandlers::new();
        let foreign_handlers = get_foreign_handlers();

        if let Some(loaded_handlers) = foreign_handlers {
            for handler in loaded_handlers.functions {
                ffi_handlers.add(handler);
            }
        }

        // events queue
        let (events_sender, events_receiver) = mpsc::unbounded_channel::<Event>();

        Vm {
            operand_stack: vec![],
            call_stack: CallStack::new(),
            memory: MemoryManager::new(),
            bytecode,
            pc: 0,
            handlers: HashMap::new(),
            ffi_handlers,
            events_queue: events_receiver,
            events_sender,
        }
    }

    pub async fn run(&mut self, args: &Vec<String>) -> VMExecutionResult {
        let debug = args.contains(&"-d".to_string());
        if debug {
            println!("last PC value: {}", self.bytecode.len());
            println!("-");
        }

        // PRELUDE
        // load builtin handlers (.map, .len)
        let raw_handlers = bootstrap_default_lib();
        let mut handlers = HashMap::new();
        for (handler_name, handler_obj) in raw_handlers {
            let obj_handle = self.memory.alloc(handler_obj);
            handlers.insert(handler_name, obj_handle);
        }
        self.handlers = handlers;

        // load builtins to scope (Byte(), Buffer)
        let builtin = builtin_functions(self);
        for (object_name, memobject) in builtin {
            let obj_handle = self.memory.alloc(memobject);
            self.call_stack
                .put_to_frame(object_name, Value::Handle(obj_handle));
        }

        // RUN
        self.run_bytecode(debug).await
    }

    fn run_bytecode<'a>(&'a mut self, debug: bool) -> BoxFuture<'a, VMExecutionResult> {
        Box::pin(async move {
            while self.pc < self.bytecode.len() {
                match Opcode::to_opcode(self.bytecode[self.pc]) {
                    Opcode::LoadConst => {
                        // parsing
                        if self.pc + 1 >= self.bytecode.len() {
                            panic!("Invalid LOAD_CONST instruction at position {}", self.pc);
                        }

                        self.pc += 1;
                        let (data_type, value_bytes) = self.get_value_length();

                        // execution
                        let (value, printable_value) = self.bytes_to_data(&data_type, &value_bytes);

                        self.push_to_stack(value, None);
                        if debug {
                            println!("LOAD_CONST <- {:?}({printable_value})", data_type);
                        }

                        self.pc += 1;
                    }
                    Opcode::LoadVar => {
                        // parsing
                        if self.pc + 1 >= self.bytecode.len() {
                            panic!("Invalid LOAD_VAR instruction at position {}", self.pc);
                        }

                        self.pc += 1;
                        // identifier
                        let (identifier_data_type, identifier_bytes) = self.get_value_length();
                        if identifier_data_type != DataType::Utf8 {
                            panic!("Identifier type should be a string encoded as utf8")
                        }
                        let identifier_name = String::from_utf8(identifier_bytes)
                            .expect("Identifier bytes should be valid UTF-8");

                        let identifier_value = self.call_stack.resolve(&identifier_name);
                        if let Some(v) = identifier_value {
                            self.push_to_stack(v, Some(identifier_name.clone()));
                            if debug {
                                println!("LOAD_VAR <- {identifier_name}");
                            }
                        } else {
                            return VMExecutionResult::terminate_with_errors(
                                VMErrorType::UndeclaredIdentifierError(identifier_name),
                                self,
                            );
                        }

                        self.pc += 1;
                    }
                    Opcode::StoreVar => {
                        // parsing
                        if self.pc + 1 >= self.bytecode.len() {
                            panic!("Invalid STORE_VAR instruction at position {}.", self.pc);
                        } else {
                            self.pc += 1;
                        }

                        // 0x00 inmutable | 0x00 mutable
                        let mutable = match self.bytecode[self.pc] {
                            0x00 => false,
                            0x01 => true,
                            _ => {
                                panic!("Invalid STORE_VAR instruction at position {}. Needed mutability property.", self.pc);
                            }
                        };
                        self.pc += 1;

                        // identifier
                        let (identifier_data_type, identifier_bytes) = self.get_value_length();
                        if identifier_data_type != DataType::Utf8 {
                            panic!("Identifier type should be a string encoded as utf8")
                        }
                        let identifier_name = String::from_utf8(identifier_bytes)
                            .expect("Identifier bytes should be valid UTF-8");

                        // release the handle if we are going to reassign
                        if let Some(i) = self.call_stack.resolve(&identifier_name) {
                            if let Value::Handle(h) = i {
                                let release_result = self.memory.release(&h);
                                if let Err(err) = release_result {
                                    return VMExecutionResult::terminate_with_errors(err, self);
                                }
                            }
                        }

                        // execution
                        let stack_stored_value = self.operand_stack.pop();
                        if let Some(v) = stack_stored_value {
                            let datatype = v.value.get_type();
                            let printable_value = v.value.to_string(self);
                            match &v.value {
                                // push_to_stack already retained the handle when it was
                                // pushed onto the operand stack; STORE_VAR just transfers
                                // that ownership into the frame (no extra retain needed)
                                Value::Handle(_) => {
                                    self.call_stack
                                        .put_to_frame(identifier_name.clone(), v.value.clone());
                                }
                                _ => {
                                    self.call_stack
                                        .put_to_frame(identifier_name.clone(), v.value);
                                }
                            }

                            if debug {
                                println!(
                                    "STORE_VAR[{}] <- {:?}({}) as {}",
                                    if mutable { "MUT" } else { "INMUT" },
                                    datatype,
                                    printable_value,
                                    identifier_name,
                                );
                            }
                        } else {
                            // todo: use self-vm errors
                            panic!("STACK UNDERFLOW")
                        }

                        self.pc += 1;
                    }
                    Opcode::Drop => {
                        self.pop_from_stack();
                        self.pc += 1;
                    }
                    Opcode::JumpIfFalse => {
                        let offset = Vm::read_offset(&self.bytecode[self.pc + 1..self.pc + 5]);
                        self.pc += 4;

                        let condition = self.pop_from_stack();
                        if condition.is_none() {
                            panic!("stack underflow");
                        };

                        let condition = condition.unwrap();
                        match condition.value.clone() {
                            Value::BoundAccess(v) => {
                                let value = v.property.as_bool(self);
                                match value {
                                    Ok(jump_if) => {
                                        if !jump_if {
                                            self.pc += offset as usize;
                                        }
                                    }
                                    Err(err) => {
                                        return VMExecutionResult::terminate_with_errors(
                                            err.error_type,
                                            self,
                                        )
                                    }
                                }
                            }
                            Value::RawValue(v) => match v {
                                RawValue::Bool(execute_if) => {
                                    if debug {
                                        println!(
                                            "JUMP_IF_FALSE <- {:?}({})",
                                            execute_if.value, offset
                                        );
                                    }
                                    if !execute_if.value {
                                        self.pc += offset as usize;
                                    }
                                }
                                _ => panic!("invalid expression type as condition to jump"),
                            },
                            _ => {
                                panic!("invalid expression type as condition to jump")
                            }
                        };

                        self.pc += 1;
                    }
                    Opcode::Jump => {
                        // execution
                        let offset = Vm::read_offset(&self.bytecode[self.pc + 1..self.pc + 5]);
                        self.pc += 4;

                        let target_pc = (self.pc as isize) + offset as isize;
                        if debug {
                            println!("JUMP <- {:?}", target_pc);
                        }
                        self.pc = target_pc as usize;
                    }
                    Opcode::Print => {
                        self.pc += 1;
                        let args = self.get_function_call_args();
                        let mut resolved_args = Vec::new();
                        for val in &args {
                            match self.value_to_string(val.clone()) {
                                Ok(v) => resolved_args.push(v),
                                Err(e) => return VMExecutionResult::terminate_with_errors(e, self),
                            }
                        }
                        print_handler(resolved_args, debug, false);
                        // args were consumed by the native handler — release their push retains
                        self.release_values(&args);
                    }
                    Opcode::Println => {
                        self.pc += 1;
                        let args = self.get_function_call_args();
                        let mut resolved_args = Vec::new();
                        for val in &args {
                            match self.value_to_string(val.clone()) {
                                Ok(v) => resolved_args.push(v),
                                Err(e) => return VMExecutionResult::terminate_with_errors(e, self),
                            }
                        }
                        print_handler(resolved_args, debug, true);
                        // args were consumed by the native handler — release their push retains
                        self.release_values(&args);
                    }
                    Opcode::FuncDec => {
                        // skip FuncDec opcode
                        if self.pc + 1 >= self.bytecode.len() {
                            panic!(
                                "Invalid FUNCTION_DECLARATION instruction at position {}.",
                                self.pc
                            );
                        } else {
                            self.pc += 1;
                        }

                        // identifier
                        let (identifier_data_type, identifier_bytes) = self.get_value_length();
                        if identifier_data_type != DataType::Utf8 {
                            panic!("Identifier type should be a string encoded as utf8")
                        }
                        let identifier_name = String::from_utf8(identifier_bytes)
                            .expect("Identifier bytes should be valid UTF-8");

                        // parameters
                        if self.pc + 4 >= self.bytecode.len() {
                            panic!("Invalid FUNC_DEC instruction at position {}", self.pc);
                        }

                        let value_bytes = &self.bytecode[self.pc + 1..self.pc + 5];
                        let parameters_length = u32::from_le_bytes(
                            value_bytes.try_into().expect("Provided value is incorrect"),
                        ) as usize;
                        // get params names from the stack
                        let params_values = self.get_stack_values(&(parameters_length as u32));
                        let params_names: Vec<String> = params_values
                            .iter()
                            .map(|val| {
                                match val {
                                    Value::Handle(r) => match self.memory.resolve(&r) {
                                        MemObject::String(s) => s.value.clone(),
                                        _ => {
                                            // TODO: use self-vm errors sytem
                                            panic!("Invalid param type for a function declaration")
                                        }
                                    },
                                    _ => {
                                        // TODO: use self-vm errors sytem
                                        panic!("Invalid param type for a function declaration")
                                    }
                                }
                            })
                            .collect();

                        self.pc += 4;

                        // handle body
                        // function body length
                        if self.pc + 4 >= self.bytecode.len() {
                            panic!("Invalid FUNC_DEC instruction at position {}", self.pc);
                        }

                        let value_bytes = &self.bytecode[self.pc + 1..self.pc + 5];
                        let body_length = u32::from_le_bytes(
                            value_bytes.try_into().expect("Provided value is incorrect"),
                        ) as usize;
                        self.pc += 4;
                        self.pc += 1; // to get next opcode

                        let body_bytecode = self.bytecode[self.pc..self.pc + body_length].to_vec();
                        self.pc += body_length;

                        // allocate function on the heap
                        let func_obj = MemObject::Function(Function::new(
                            identifier_name.clone(),
                            params_names,
                            Engine::Bytecode(body_bytecode),
                        ));
                        let func_handle = self.memory.alloc(func_obj);

                        // make accesible on the current context
                        self.call_stack
                            .put_to_frame(identifier_name, Value::Handle(func_handle));
                    }
                    Opcode::StructDec => {
                        // skip StructDec opcode
                        self.pc += 1;

                        // identifier
                        let (identifier_data_type, identifier_bytes) = self.get_value_length();
                        if identifier_data_type != DataType::Utf8 {
                            // TODO: use self-vm errors
                            panic!("Identifier type should be a string encoded as utf8")
                        }

                        // TODO: use self-vm errors
                        let identifier_name = String::from_utf8(identifier_bytes)
                            .expect("Identifier bytes should be valid UTF-8");

                        // read fields number
                        self.pc += 1;
                        let fields_num = Vm::read_offset(&self.bytecode[self.pc..self.pc + 4]);
                        self.pc += 4;

                        // struct fields [raw_string][type][raw_string][type]
                        //               (x)B        1B    (x)B        1B
                        let mut counter = 0;
                        let mut fields = vec![];
                        while counter < fields_num {
                            // field
                            let (field_data_type, field_bytes) = self.get_value_length();
                            if field_data_type != DataType::Utf8 {
                                // TODO: use self-vm errors
                                panic!("Identifier type should be a string encoded as utf8")
                            }
                            let field_name = String::from_utf8(field_bytes)
                                .expect("Field bytes should be valid UTF-8"); // TODO: use self-vm errors
                            self.pc += 1;

                            // annotation
                            let annotation = DataType::to_opcode(self.bytecode[self.pc]);
                            self.pc += 1;

                            fields.push((field_name, annotation));
                            counter += 1;
                        }

                        // struct declaration
                        let struct_declaration =
                            StructDeclaration::new(identifier_name.clone(), fields);
                        // push to declaration heap
                        let heap_handle = self
                            .memory
                            .alloc(MemObject::StructDeclaration(struct_declaration));
                        self.call_stack
                            .put_to_frame(identifier_name, Value::Handle(heap_handle));
                    }
                    Opcode::GetProperty => {
                        let values = self.get_stack_values(&2);
                        let (object_handle, property_handle) = match (&values[0], &values[1]) {
                            (Value::Handle(obj_handle), Value::Handle(prop_handle)) => {
                                (obj_handle.clone(), prop_handle.clone())
                            }
                            // nested property acess
                            (Value::BoundAccess(bound_access), Value::Handle(prop_handle)) => {
                                let property_handle = match bound_access.property.as_handle() {
                                    Ok(v) => v.clone(),
                                    Err(err) => {
                                        panic!("error on get property on nested levels {:#?}", err);
                                    }
                                };

                                (property_handle, prop_handle.clone())
                            }
                            // TODO: use self-vm errors
                            // here we should handle if a function returns an
                            // nothing istead of a struct
                            _ => {
                                println!("values: {:#?}", values);
                                panic!("Expected two Handle values for <get_property> opcode")
                            }
                        };

                        let object = self.memory.resolve(&object_handle);
                        let property = self.memory.resolve(&property_handle);

                        if debug {
                            println!(
                                "GET_PROPERTY <- {}({:?})",
                                object.to_string(self),
                                property.to_string(self)
                            );
                        }

                        if let MemObject::String(property_key) = property {
                            match object {
                                MemObject::StructLiteral(x) => {
                                    let value = x.property_access(&property_key.value);
                                    if let Some(prop) = value {
                                        let bound_access =
                                            BoundAccess::new(object_handle.clone(), Box::new(prop));
                                        self.push_to_stack(
                                            Value::BoundAccess(bound_access),
                                            Some(object.to_string(self)),
                                        );
                                    } else {
                                        return VMExecutionResult::terminate_with_errors(
                                            VMErrorType::Struct(StructError::FieldNotFound {
                                                field: property_key.to_string(),
                                                struct_type: object.to_string(self),
                                            }),
                                            self,
                                        );
                                    }
                                }
                                MemObject::NativeStruct(x) => {
                                    let value = x.property_access(&property_key.value);
                                    if let Some(prop) = value {
                                        let bound_access =
                                            BoundAccess::new(object_handle.clone(), Box::new(prop));
                                        self.push_to_stack(
                                            Value::BoundAccess(bound_access),
                                            Some(object.to_string(self)),
                                        );
                                    } else {
                                        return VMExecutionResult::terminate_with_errors(
                                            VMErrorType::Struct(StructError::FieldNotFound {
                                                field: property_key.to_string(),
                                                struct_type: object.to_string(self),
                                            }),
                                            self,
                                        );
                                    }
                                }
                                MemObject::Vector(x) => {
                                    let value = x.property_access(&property_key.value);
                                    if let Some(prop) = value {
                                        let bound_access =
                                            BoundAccess::new(object_handle.clone(), Box::new(prop));
                                        self.push_to_stack(
                                            Value::BoundAccess(bound_access),
                                            Some(object.to_string(self)),
                                        );
                                    } else {
                                        return VMExecutionResult::terminate_with_errors(
                                            VMErrorType::Struct(StructError::FieldNotFound {
                                                field: property_key.to_string(),
                                                struct_type: object.to_string(self),
                                            }),
                                            self,
                                        );
                                    }
                                }
                                MemObject::String(x) => {
                                    let value = x.property_access(&property_key.value);
                                    if let Some(prop) = value {
                                        let bound_access =
                                            BoundAccess::new(object_handle.clone(), Box::new(prop));
                                        self.push_to_stack(
                                            Value::BoundAccess(bound_access),
                                            Some(object.to_string(self)),
                                        );
                                    } else {
                                        return VMExecutionResult::terminate_with_errors(
                                            VMErrorType::Struct(StructError::FieldNotFound {
                                                field: property_key.to_string(),
                                                struct_type: object.to_string(self),
                                            }),
                                            self,
                                        );
                                    }
                                }
                                _ => {
                                    panic!(
                                        "<get_property> opcode must be used on a Struct like type"
                                    )
                                }
                            }
                        } else {
                            // TODO: use self-vm errors
                            panic!("Struct literal field must be indexed by string")
                        }

                        // Release the push_to_stack retains from the two raw-popped values.
                        // The BoundAccess now holds the object reference; it doesn't own
                        // an extra retain (push_to_stack doesn't retain BoundAccess).
                        for v in &values {
                            if let Value::Handle(h) = v {
                                let _ = self.memory.release(h);
                            }
                        }

                        self.pc += 1;
                    }
                    Opcode::GetIndex => {
                        let values = self.get_stack_values(&2);
                        let object_val = &values[0];
                        let index_val = &values[1];

                        // resolve index as usize
                        let index = match self.unwrap_bound_access(index_val.clone()).as_usize(self)
                        {
                            Ok(v) => v,
                            Err(err) => {
                                return VMExecutionResult::terminate_with_errors(err.error_type, self)
                            }
                        };

                        // resolve object (Vector or String)
                        let unwrapped_object = self.unwrap_bound_access(object_val.clone());
                        let (object_handle, resolved_object) = match unwrapped_object {
                            Value::Handle(h) => (h.clone(), self.memory.resolve(&h)),
                            _ => {
                                let error = VMErrorType::TypeMismatch {
                                    expected: "Vector or String".to_string(),
                                    received: unwrapped_object.get_type(),
                                };
                                return VMExecutionResult::terminate_with_errors(error, self);
                            }
                        };

                        match resolved_object {
                            MemObject::Vector(v) => {
                                if index >= v.elements.len() {
                                    return VMExecutionResult::terminate_with_errors(
                                        VMErrorType::Vector(VectorError::IndexOutOfBounds {
                                            index,
                                            length: v.elements.len(),
                                        }),
                                        self,
                                    );
                                }
                                let val = v.elements[index].clone();
                                let bound_access =
                                    BoundAccess::new(object_handle.clone(), Box::new(val));
                                self.push_to_stack(
                                    Value::BoundAccess(bound_access),
                                    Some(format!("{}[{}]", object_val.to_string(self), index)),
                                );
                            }
                            MemObject::String(s) => {
                                if index >= s.value.len() {
                                    return VMExecutionResult::terminate_with_errors(
                                        VMErrorType::Vector(VectorError::IndexOutOfBounds {
                                            index,
                                            length: s.value.len(),
                                        }),
                                        self,
                                    );
                                }
                                let char_str = s.value.chars().nth(index).unwrap().to_string();
                                let char_handle = crate::std::heap_utils::put_string(self, char_str);

                                self.push_to_stack(
                                    Value::Handle(char_handle),
                                    Some(format!("{}[{}]", object_val.to_string(self), index)),
                                );
                            }
                            _ => {
                                let error = VMErrorType::TypeMismatch {
                                    expected: "Vector or String".to_string(),
                                    received: resolved_object.get_type(),
                                };
                                return VMExecutionResult::terminate_with_errors(error, self);
                            }
                        }

                        // Release the push_to_stack retain for the object handle (raw-popped)
                        if let Value::Handle(h) = &values[0] {
                            let _ = self.memory.release(h);
                        }

                        self.pc += 1;
                    }
                    Opcode::Call => {
                        self.pc += 1;
                        let args = self.get_function_call_args();
                        let callee_value = self.get_stack_values(&1);

                        // For named function calls the callee is a Handle(String) that was
                        // pushed via push_to_stack (retained). We raw-pop it here, so we
                        // must release that retain after the call. Save a copy now, before
                        // the handle is moved into the match arms below.
                        // For BoundAccess calls, GetProperty already released the object
                        // handle's push retain — nothing extra to release here.
                        let caller_handle_to_release = match &callee_value[0] {
                            Value::Handle(h) => Some(h.clone()),
                            _ => None,
                        };

                        let ((caller_obj, caller_handle), callee_handle): (
                            (&MemObject, Handle),
                            Option<Handle>,
                        ) = match callee_value[0].clone() {
                            Value::Handle(handle) => ((self.memory.resolve(&handle), handle), None),
                            Value::BoundAccess(b) => {
                                if let Value::Handle(callee_handle) = b.property.as_ref() {
                                    (
                                        (self.memory.resolve(&b.object), b.object),
                                        Some(callee_handle.clone()),
                                    )
                                } else {
                                    // nested bound accesses
                                    panic!("Invalid type for callee string")
                                }
                            }
                            _ => {
                                // TODO: use self-vm error system
                                panic!("Invalid type for callee string")
                            }
                        };

                        match caller_obj {
                            // FOR NAMED FUNCTIONS ACCESS
                            MemObject::String(identifier_name) => {
                                if debug {
                                    println!("CALL -> {}", identifier_name.to_string())
                                };
                                match identifier_name.value.as_str() {
                                    // BUILTIN FUNCTIONS
                                    "type" => {
                                        let var_type = match &args[0] {
                                            Value::BoundAccess(_) => {
                                                todo!("Implement bound acces lookup")
                                            }
                                            Value::Handle(h) => self.memory.resolve(&h).get_type(),
                                            Value::RawValue(r) => match r {
                                                RawValue::Bool(v) => "bool".to_string(),
                                                RawValue::Utf8(v) => "string".to_string(),
                                                RawValue::I32(v) => "number".to_string(),
                                                RawValue::I64(v) => "number".to_string(),
                                                RawValue::U32(v) => "number".to_string(),
                                                RawValue::U64(v) => "number".to_string(),
                                                RawValue::F64(v) => "number".to_string(),
                                                RawValue::Byte(v) => "byte".to_string(),
                                                RawValue::Nothing => "nothing".to_string(),
                                            },
                                            x => todo!("Implement {:#?} value in type function", x),
                                        };

                                        self.push_to_stack(
                                            Value::RawValue(RawValue::Utf8(Utf8::new(var_type))),
                                            Some("type".to_string()),
                                        );
                                    }
                                    // RUNTIME DEFINED FUNCTIONS
                                    _ => {
                                        // get the identifier from the heap for calling runtime defined functions
                                        let value = if let Some(value) =
                                            self.call_stack.resolve(&identifier_name.value)
                                        {
                                            value

                                        // calling string object members
                                        } else if let Some(_callee_handle) = callee_handle {
                                            Value::Handle(_callee_handle)
                                        } else {
                                            return VMExecutionResult::terminate_with_errors(
                                                VMErrorType::UndeclaredIdentifierError(
                                                    identifier_name.value.clone(),
                                                ),
                                                self,
                                            );
                                        };

                                        match value {
                                            Value::Handle(v) => {
                                                // clone heap_object to be able to mutate the
                                                // vm state
                                                let heap_object = self.memory.resolve(&v);
                                                if let MemObject::Function(func) = heap_object {
                                                    let func = func.clone();
                                                    let exec_result = self
                                                        .run_function(
                                                            &func,
                                                            Some(caller_handle),
                                                            args.clone(),
                                                            debug,
                                                        )
                                                        .await;
                                                    if exec_result.error.is_some() {
                                                        return VMExecutionResult::terminate_with_errors(
                                                                exec_result.error.unwrap().error_type,
                                                                self
                                                            );
                                                    }
                                                    if let Some(returned_value) = exec_result.result {
                                                        // Direct push: the callee's push_to_stack
                                                        // already retained this handle; no extra retain
                                                        self.operand_stack.push(OperandsStackValue {
                                                            value: returned_value,
                                                            origin: Some(func.identifier.clone()),
                                                        });
                                                    }
                                                } else {
                                                    return VMExecutionResult::terminate_with_errors(
                                                    VMErrorType::NotCallableError(
                                                        identifier_name.value.clone(),
                                                    ),
                                                    self,
                                                );
                                                }
                                            }
                                            _ => {
                                                return VMExecutionResult::terminate_with_errors(
                                                    VMErrorType::NotCallableError(
                                                        identifier_name.value.clone(),
                                                    ),
                                                    self,
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            // FOR STRUCTS CALLABLE MEMBERS
                            MemObject::StructLiteral(caller) => {
                                let callee_handle = if let Some(c) = callee_handle {
                                    c
                                } else {
                                    // TODO: use self-vm error system
                                    panic!("callee is not defined for a struct as function caller")
                                };

                                let callee = self.memory.resolve(&callee_handle);
                                if let MemObject::Function(func) = callee {
                                    let func = func.clone();
                                    let exec_result = self
                                        .run_function(
                                            &func,
                                            Some(caller_handle),
                                            args.clone(),
                                            debug,
                                        )
                                        .await;
                                    if exec_result.error.is_some() {
                                        return VMExecutionResult::terminate_with_errors(
                                            exec_result.error.unwrap().error_type,
                                            self,
                                        );
                                    }
                                    if let Some(returned_value) = exec_result.result {
                                        self.operand_stack.push(OperandsStackValue {
                                            value: returned_value,
                                            origin: Some(func.identifier.clone()),
                                        });
                                    }
                                } else {
                                    return VMExecutionResult::terminate_with_errors(
                                        VMErrorType::NotCallableError(caller.struct_type.clone()),
                                        self,
                                    );
                                }
                            }

                            // FOR NATIVE_STRUCTS CALLABLE MEMBERS
                            MemObject::NativeStruct(caller) => {
                                let callee_handle = if let Some(c) = callee_handle {
                                    c
                                } else {
                                    // TODO: use self-vm error system
                                    panic!("callee is not defined for a struct as function caller")
                                };

                                let callee = self.memory.resolve(&callee_handle);
                                if let MemObject::Function(func) = callee {
                                    let func = func.clone();
                                    let exec_result =
                                    // instead of none callee_handle
                                    self.run_function(&func, Some(caller_handle), args.clone(), debug).await;
                                    if exec_result.error.is_some() {
                                        return VMExecutionResult::terminate_with_errors(
                                            exec_result.error.unwrap().error_type,
                                            self,
                                        );
                                    }
                                    if let Some(returned_value) = exec_result.result {
                                        self.operand_stack.push(OperandsStackValue {
                                            value: returned_value,
                                            origin: Some(func.identifier.clone()),
                                        });
                                    }
                                } else {
                                    return VMExecutionResult::terminate_with_errors(
                                        VMErrorType::NotCallableError(caller.to_string(self)),
                                        self,
                                    );
                                }
                            }

                            // FOR VECTOR CALLABLE MEMBERS
                            MemObject::Vector(caller) => {
                                let callee_handle = if let Some(c) = callee_handle {
                                    c
                                } else {
                                    // TODO: use self-vm error system
                                    panic!("callee is not defined for a vec as a function caller")
                                };

                                let callee = self.memory.resolve(&callee_handle);
                                if let MemObject::Function(func) = callee {
                                    let func = func.clone();
                                    let exec_result = self
                                        .run_function(
                                            &func,
                                            Some(caller_handle),
                                            args.clone(),
                                            debug,
                                        )
                                        .await;
                                    if exec_result.error.is_some() {
                                        return VMExecutionResult::terminate_with_errors(
                                            exec_result.error.unwrap().error_type,
                                            self,
                                        );
                                    }
                                    if let Some(returned_value) = exec_result.result {
                                        self.operand_stack.push(OperandsStackValue {
                                            value: returned_value,
                                            origin: Some(func.identifier.clone()),
                                        });
                                    }
                                } else {
                                    return VMExecutionResult::terminate_with_errors(
                                        VMErrorType::NotCallableError(caller.to_string(self)),
                                        self,
                                    );
                                }
                            }
                            _ => {
                                panic!("Invalid type for callee string")
                            }
                        }

                        // Release the function-name string retain from push_to_stack.
                        // (For BoundAccess calls this is None — GetProperty already handled it.)
                        if let Some(h) = caller_handle_to_release {
                            let _ = self.memory.release(&h);
                        }
                    }
                    Opcode::Import => {
                        let values = self.get_stack_values(&1);
                        let module_name_value = values[0].clone();
                        let mod_bytecode_length =
                            Vm::read_offset(&self.bytecode[self.pc + 1..self.pc + 5]);
                        self.pc += 4;

                        if let Value::Handle(mod_handle) = module_name_value {
                            let module_name = self.memory.resolve(&mod_handle).to_string(self);
                            let native_module = get_native_module_type(module_name.as_str());
                            // native module
                            if let Some(nm) = native_module {
                                // load native module fields
                                let module_def = generate_native_module(nm);
                                let mut module_fields = HashMap::new();
                                for field in module_def.1 {
                                    let field_handle = self.memory.alloc(field.1);
                                    module_fields.insert(field.0, Value::Handle(field_handle));
                                }

                                // create the native module struct
                                let module_struct = StructLiteral::new(module_def.0, module_fields, self);
                                let module_struct_handle =
                                    self.memory.alloc(MemObject::StructLiteral(module_struct));

                                // retain once so the frame pop can release it correctly
                                let _ = self.memory.retain(&module_struct_handle);
                                self.call_stack.put_to_frame(
                                    module_name.to_string(),
                                    Value::Handle(module_struct_handle),
                                );
                            } else {
                                // custom module
                                let mod_name = Path::new(&module_name)
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("unknown");
                                let mod_bytecode = &self.bytecode
                                    [self.pc + 1..(self.pc + (mod_bytecode_length as usize)) + 1];
                                self.pc += mod_bytecode_length as usize;
                                let exec_result = self
                                    .run_module(&mod_name.to_string(), mod_bytecode.to_vec(), debug)
                                    .await;
                                if exec_result.error.is_some() {
                                    return exec_result;
                                }

                                // if members exported, add them to the scope
                                if let Some(result) = exec_result.result {
                                    if let Value::Handle(r) = result {
                                        // retain once so the frame pop can release it correctly
                                        let _ = self.memory.retain(&r);
                                        self.call_stack
                                            .put_to_frame(mod_name.to_string(), Value::Handle(r));
                                    }
                                }
                            }
                            // release the push retain for the module name string
                            // (it was a temporary string on the operand stack)
                            let _ = self.memory.release(&mod_handle);
                        } else {
                            // TODO: use self-vm errors system
                            panic!("invalid value type as module name for import")
                        }

                        self.pc += 1;
                    }
                    Opcode::Export => {
                        let arg_ref = self.get_stack_values(&1)[0].clone();
                        if let Value::Handle(r) = arg_ref.clone() {
                            let arg = self.memory.resolve(&r);
                            if let MemObject::String(s) = arg {
                                if debug {
                                    println!("EXPORT -> {}", s.value)
                                }
                                self.call_stack.add_export(s.to_string());
                            } else {
                                return VMExecutionResult::terminate_with_errors(
                                    VMErrorType::ExportInvalidMemberType,
                                    self,
                                );
                            }
                        } else {
                            return VMExecutionResult::terminate_with_errors(
                                VMErrorType::ExportInvalidMemberType,
                                self,
                            );
                        }
                        self.pc += 1;
                    }
                    Opcode::Return => {
                        // get_stack_values transfers ownership without releasing:
                        // the push_to_stack retain travels with the value to the caller
                        let return_value = self.get_stack_values(&1)[0].clone();
                        return VMExecutionResult::terminate(Some(return_value));
                    }
                    Opcode::Add => {
                        // execution
                        // Raw pop — handles are still alive during run_binary_expression;
                        // run_binary_expression releases them after the op completes.
                        let right_operand = self.operand_stack.pop();
                        let left_operand = self.operand_stack.pop();

                        if left_operand.is_none() || right_operand.is_none() {
                            panic!("Operands stack underflow");
                        };

                        let operands_stack_values = (left_operand.unwrap(), right_operand.unwrap());

                        let error = self.run_binary_expression("+", operands_stack_values);
                        if let Some(err) = error {
                            return VMExecutionResult::terminate_with_errors(err, self);
                        }

                        self.pc += 1;
                    }
                    Opcode::Substract => {
                        // execution
                        // Raw pop — handles are still alive during run_binary_expression;
                        // run_binary_expression releases them after the op completes.
                        let right_operand = self.operand_stack.pop();
                        let left_operand = self.operand_stack.pop();

                        if left_operand.is_none() || right_operand.is_none() {
                            panic!("Operands stack underflow");
                        };

                        let operands_stack_values = (left_operand.unwrap(), right_operand.unwrap());

                        let error = self.run_binary_expression("-", operands_stack_values);
                        if let Some(err) = error {
                            return VMExecutionResult::terminate_with_errors(err, self);
                        }

                        self.pc += 1;
                    }
                    Opcode::Multiply => {
                        // execution
                        // Raw pop — handles are still alive during run_binary_expression;
                        // run_binary_expression releases them after the op completes.
                        let right_operand = self.operand_stack.pop();
                        let left_operand = self.operand_stack.pop();

                        if left_operand.is_none() || right_operand.is_none() {
                            panic!("Operands stack underflow");
                        };

                        let operands_stack_values = (left_operand.unwrap(), right_operand.unwrap());

                        let error = self.run_binary_expression("*", operands_stack_values);
                        if let Some(err) = error {
                            return VMExecutionResult::terminate_with_errors(err, self);
                        }

                        self.pc += 1;
                    }
                    Opcode::Divide => {
                        // execution
                        // Raw pop — handles are still alive during run_binary_expression;
                        // run_binary_expression releases them after the op completes.
                        let right_operand = self.operand_stack.pop();
                        let left_operand = self.operand_stack.pop();

                        if left_operand.is_none() || right_operand.is_none() {
                            panic!("Operands stack underflow");
                        };

                        let operands_stack_values = (left_operand.unwrap(), right_operand.unwrap());

                        let error = self.run_binary_expression("/", operands_stack_values);
                        if let Some(err) = error {
                            return VMExecutionResult::terminate_with_errors(err, self);
                        }

                        self.pc += 1;
                    }
                    Opcode::GreaterThan => {
                        // execution
                        // Raw pop — handles are still alive during run_binary_expression;
                        // run_binary_expression releases them after the op completes.
                        let right_operand = self.operand_stack.pop();
                        let left_operand = self.operand_stack.pop();

                        if left_operand.is_none() || right_operand.is_none() {
                            panic!("Operands stack underflow");
                        };

                        let operands_stack_values = (left_operand.unwrap(), right_operand.unwrap());

                        let error = self.run_binary_expression(">", operands_stack_values);
                        if let Some(err) = error {
                            return VMExecutionResult::terminate_with_errors(err, self);
                        }

                        self.pc += 1;
                    }
                    Opcode::LessThan => {
                        // execution
                        // Raw pop — handles are still alive during run_binary_expression;
                        // run_binary_expression releases them after the op completes.
                        let right_operand = self.operand_stack.pop();
                        let left_operand = self.operand_stack.pop();

                        if left_operand.is_none() || right_operand.is_none() {
                            panic!("Operands stack underflow");
                        };

                        let operands_stack_values = (left_operand.unwrap(), right_operand.unwrap());

                        let error = self.run_binary_expression("<", operands_stack_values);
                        if let Some(err) = error {
                            return VMExecutionResult::terminate_with_errors(err, self);
                        }

                        self.pc += 1;
                    }
                    Opcode::Equals => {
                        // execution
                        // Raw pop — handles are still alive during run_binary_expression;
                        // run_binary_expression releases them after the op completes.
                        let right_operand = self.operand_stack.pop();
                        let left_operand = self.operand_stack.pop();

                        if left_operand.is_none() || right_operand.is_none() {
                            panic!("Operands stack underflow");
                        };

                        let operands_stack_values = (left_operand.unwrap(), right_operand.unwrap());

                        let error = self.run_binary_expression("==", operands_stack_values);
                        if let Some(err) = error {
                            return VMExecutionResult::terminate_with_errors(err, self);
                        }

                        self.pc += 1;
                    }
                    Opcode::NotEquals => {
                        // execution
                        // Raw pop — handles are still alive during run_binary_expression;
                        // run_binary_expression releases them after the op completes.
                        let right_operand = self.operand_stack.pop();
                        let left_operand = self.operand_stack.pop();

                        if left_operand.is_none() || right_operand.is_none() {
                            panic!("Operands stack underflow");
                        };

                        let operands_stack_values = (left_operand.unwrap(), right_operand.unwrap());

                        let error = self.run_binary_expression("!=", operands_stack_values);
                        if let Some(err) = error {
                            return VMExecutionResult::terminate_with_errors(err, self);
                        }

                        self.pc += 1;
                    }
                    Opcode::UnaryNegation => {
                        let operand = self.pop_from_stack();
                        if operand.is_none() {
                            panic!("stack underflow");
                        };

                        let operand = operand.unwrap();
                        let unwraped_operand = self.unwrap_bound_access(operand.value.clone());
                        let value = unwraped_operand.as_bool(self);
                        match value {
                            Ok(v) => {
                                self.push_to_stack(
                                    Value::RawValue(RawValue::Bool(Bool::new(!v))),
                                    None,
                                );
                            }
                            Err(err) => {
                                return VMExecutionResult::terminate_with_errors(
                                    err.error_type,
                                    self,
                                )
                            }
                        }

                        self.pc += 1;
                    }
                    Opcode::FFI_Call => {
                        self.pc += 1; // consume call opcode
                        let args = self.get_function_call_args();
                        let mut resolved_args = Vec::new();
                        for val in args {
                            match self.value_to_string(val) {
                                Ok(v) => resolved_args.push(v),
                                Err(e) => return VMExecutionResult::terminate_with_errors(e, self),
                            }
                        }
                        if debug {
                            println!("CALL -> {}", resolved_args[0].to_string())
                        }
                        call_handler(&self.ffi_handlers, resolved_args);
                    }
                    _ => {
                        println!("unhandled opcode");
                        self.pc += 1;
                    }
                };

                // drain vm events
                // if we only get one event on each execution
                // could happen that we have more events than
                // iterations on the vm
                self.drain_events().await;
            }

            VMExecutionResult::terminate(None)
        })
    }

    fn run_binary_expression(
        &mut self,
        operator: &str,
        operands: (OperandsStackValue, OperandsStackValue),
    ) -> Option<VMErrorType> {
        let left = operands.0;
        let right = operands.1;

        // unwrap BoundAccess values to their underlying values
        // This enables expressions like "{" + var.property + "}"
        let left_value = self.unwrap_bound_access(left.value.clone());
        let right_value = self.unwrap_bound_access(right.value.clone());

        let value: Value;
        // cloned here, to be able to use later on
        // different VMErrors
        match (left_value, right_value.clone()) {
            (Value::RawValue(l), Value::RawValue(r)) => {
                let result_value = match (l, r) {
                    (RawValue::I32(l), RawValue::I32(r)) => match operator {
                        "+" => RawValue::I32(I32::new(l.value + r.value)),
                        "-" => RawValue::I32(I32::new(l.value - r.value)),
                        "*" => RawValue::I32(I32::new(l.value * r.value)),
                        "/" => RawValue::I32(I32::new(l.value / r.value)),
                        ">" => RawValue::Bool(Bool::new(l.value > r.value)),
                        "<" => RawValue::Bool(Bool::new(l.value < r.value)),
                        "==" => RawValue::Bool(Bool::new(l.value == r.value)),
                        "!=" => RawValue::Bool(Bool::new(l.value != r.value)),
                        _ => {
                            panic!("operator not implemented")
                        }
                    },
                    (RawValue::I64(l), RawValue::I64(r)) => match operator {
                        "+" => RawValue::I64(I64::new(l.value + r.value)),
                        "-" => RawValue::I64(I64::new(l.value - r.value)),
                        "*" => RawValue::I64(I64::new(l.value * r.value)),
                        "/" => RawValue::I64(I64::new(l.value / r.value)),
                        ">" => RawValue::Bool(Bool::new(l.value > r.value)),
                        "<" => RawValue::Bool(Bool::new(l.value < r.value)),
                        "==" => RawValue::Bool(Bool::new(l.value == r.value)),
                        "!=" => RawValue::Bool(Bool::new(l.value != r.value)),
                        _ => {
                            panic!("operator not implemented in i64")
                        }
                    },
                    (RawValue::U32(l), RawValue::U32(r)) => match operator {
                        "+" => RawValue::U32(U32::new(l.value + r.value)),
                        "-" => RawValue::U32(U32::new(l.value - r.value)),
                        "*" => RawValue::U32(U32::new(l.value * r.value)),
                        "/" => RawValue::U32(U32::new(l.value / r.value)),
                        ">" => RawValue::Bool(Bool::new(l.value > r.value)),
                        "<" => RawValue::Bool(Bool::new(l.value < r.value)),
                        "==" => RawValue::Bool(Bool::new(l.value == r.value)),
                        "!=" => RawValue::Bool(Bool::new(l.value != r.value)),
                        _ => {
                            panic!("operator not implemented in u32")
                        }
                    },
                    (RawValue::U64(l), RawValue::U64(r)) => match operator {
                        "+" => RawValue::U64(U64::new(l.value + r.value)),
                        "-" => RawValue::U64(U64::new(l.value - r.value)),
                        "*" => RawValue::U64(U64::new(l.value * r.value)),
                        "/" => RawValue::U64(U64::new(l.value / r.value)),
                        ">" => RawValue::Bool(Bool::new(l.value > r.value)),
                        "<" => RawValue::Bool(Bool::new(l.value < r.value)),
                        "==" => RawValue::Bool(Bool::new(l.value == r.value)),
                        "!=" => RawValue::Bool(Bool::new(l.value != r.value)),
                        _ => {
                            panic!("operator not implemented in u64")
                        }
                    },
                    (RawValue::F64(l), RawValue::F64(r)) => match operator {
                        "+" => RawValue::F64(F64::new(l.value + r.value)),
                        "-" => RawValue::F64(F64::new(l.value - r.value)),
                        "*" => RawValue::F64(F64::new(l.value * r.value)),
                        "/" => RawValue::F64(F64::new(l.value / r.value)),
                        ">" => RawValue::Bool(Bool::new(l.value > r.value)),
                        "<" => RawValue::Bool(Bool::new(l.value < r.value)),
                        "==" => RawValue::Bool(Bool::new(l.value == r.value)),
                        "!=" => RawValue::Bool(Bool::new(l.value != r.value)),
                        _ => {
                            panic!("operator not implemented in f64")
                        }
                    },
                    (RawValue::Nothing, RawValue::Nothing) => {
                        return Some(VMErrorType::InvalidBinaryOperation(
                            InvalidBinaryOperation {
                                left: DataType::Nothing,
                                right: DataType::Nothing,
                                operator: operator.to_string(),
                            },
                        ))
                    }
                    (RawValue::Utf8(l), RawValue::Utf8(r)) => match operator {
                        "==" => RawValue::Bool(Bool::new(l.value == r.value)),
                        "!=" => RawValue::Bool(Bool::new(l.value != r.value)),
                        _ => {
                            return Some(VMErrorType::InvalidBinaryOperation(
                                InvalidBinaryOperation {
                                    left: DataType::Utf8,
                                    right: DataType::Utf8,
                                    operator: operator.to_string(),
                                },
                            ))
                        }
                    },
                    (RawValue::Bool(_), RawValue::Bool(_)) => {
                        return Some(VMErrorType::InvalidBinaryOperation(
                            InvalidBinaryOperation {
                                left: DataType::Bool,
                                right: DataType::Bool,
                                operator: operator.to_string(),
                            },
                        ))
                    }
                    _ => return Some(VMErrorType::TypeCoercionError(right)),
                };

                value = Value::RawValue(result_value);
            }
            (Value::Handle(l), Value::Handle(r)) => {
                // here implement binary operations between different
                // types once the Handle is resolved to the actual value
                let l_heap_object = self.memory.resolve(&l);
                let r_heap_object = self.memory.resolve(&r);

                let result_value = match (l_heap_object, r_heap_object) {
                    (MemObject::String(left_string), MemObject::String(right_string)) => {
                        match operator {
                            "+" => {
                                let result_string =
                                    format!("{}{}", left_string.value, right_string.value);
                                Value::Handle(put_string(self, result_string))
                            }
                            "==" => Value::RawValue(RawValue::Bool(Bool::new(
                                left_string.value == right_string.value,
                            ))),
                            "!=" => Value::RawValue(RawValue::Bool(Bool::new(
                                left_string.value != right_string.value,
                            ))),
                            _ => {
                                return Some(VMErrorType::InvalidBinaryOperation(
                                    InvalidBinaryOperation {
                                        left: DataType::Utf8,
                                        right: DataType::Utf8,
                                        operator: operator.to_string(),
                                    },
                                ))
                            }
                        }
                    } // when more heap type exists implement here a
                    _ => {
                        return Some(VMErrorType::InvalidBinaryOperation(
                            // we should (probably) implement a system to refer to functions
                            // data type either creating a new type RuntimeType or extending
                            // DataType
                            InvalidBinaryOperation {
                                left: DataType::Unknown,
                                right: DataType::Unknown,
                                operator: operator.to_string(),
                            },
                        ));
                    }
                };

                value = result_value;
            }
            (Value::Handle(l), Value::RawValue(r)) => {
                // allow Handle(String) op RawValue for +, ==, !=
                let l_heap_object = self.memory.resolve(&l);

                if l_heap_object.get_type() != "string" {
                    return Some(VMErrorType::TypeCoercionError(right));
                }

                let l_str = l_heap_object.to_string(self);
                match operator {
                    "+" => {
                        // String + any numeric/utf8 raw value → string concatenation
                        let result_string = format!("{}{}", l_str, r.to_string());
                        value = Value::Handle(put_string(self, result_string));
                    }
                    "==" => {
                        value = Value::RawValue(RawValue::Bool(Bool::new(
                            l_str == r.to_string(),
                        )))
                    }
                    "!=" => {
                        value = Value::RawValue(RawValue::Bool(Bool::new(
                            l_str != r.to_string(),
                        )))
                    }
                    _ => {
                        return Some(VMErrorType::TypeCoercionError(right));
                    }
                };
            }
            (Value::RawValue(l), Value::Handle(r)) => {
                // allow RawValue op Handle(String) for +, ==, !=
                let r_heap_object = self.memory.resolve(&r);

                if r_heap_object.get_type() != "string" {
                    return Some(VMErrorType::TypeCoercionError(right));
                }

                let r_str = r_heap_object.to_string(self);
                match operator {
                    "+" => {
                        // any numeric/utf8 raw value + String → string concatenation
                        let result_string = format!("{}{}", l.to_string(), r_str);
                        value = Value::Handle(put_string(self, result_string));
                    }
                    "==" => {
                        value = Value::RawValue(RawValue::Bool(Bool::new(
                            r_str == l.to_string(),
                        )))
                    }
                    "!=" => {
                        value = Value::RawValue(RawValue::Bool(Bool::new(
                            r_str != l.to_string(),
                        )))
                    }
                    _ => {
                        return Some(VMErrorType::TypeCoercionError(right));
                    }
                };
            }
            _ => {
                panic!("invalid Value type for a binary expression")
            }
        }

        self.push_to_stack(value, None);

        // Release operand handle retains now that the op has consumed them
        if let Value::Handle(h) = &left.value {
            let _ = self.memory.release(h);
        }
        if let Value::Handle(h) = &right.value {
            let _ = self.memory.release(h);
        }

        None
    }

    async fn run_module(
        &mut self,
        mod_name: &String,
        mod_bytecode: Vec<u8>,
        debug: bool,
    ) -> VMExecutionResult {
        let return_pc = self.pc;
        let main_bytecode = std::mem::take(&mut self.bytecode);

        self.call_stack.push();
        self.bytecode = mod_bytecode.clone();
        self.pc = 0;
        let mut mod_exec_result = self.run_bytecode(debug).await;

        // recover state after execution
        let mod_frame = self.call_stack.pop(); // here we should lookup the exports and store on a struct, then, return that struct on the VMExecutionResult
        if let Some(mut frame) = mod_frame {
            let exported_members = frame.get_exports();
            let exports_struct = StructLiteral::new(mod_name.to_string(), exported_members, self);
            let exports_handle = self.memory.alloc(MemObject::StructLiteral(exports_struct));

            mod_exec_result.result = Some(Value::Handle(exports_handle));
        }
        self.pc = return_pc;
        self.bytecode = main_bytecode;

        mod_exec_result
    }

    pub async fn run_function(
        &mut self,
        func: &Function,
        caller: Option<Handle>,
        args: Vec<Value>,
        debug: bool,
    ) -> VMExecutionResult {
        let execution_result = match &func.engine {
            Engine::Bytecode(bytecode) => {
                let return_pc = self.pc;
                let main_bytecode = std::mem::take(&mut self.bytecode);
                // TODO:
                // probably in the future we could make that each
                // stack frame has its own operands stack
                let prev_operand_stack = std::mem::take(&mut self.operand_stack);

                self.call_stack.push();
                for (index, param) in func.parameters.iter().enumerate() {
                    if index < args.len() {
                        self.call_stack
                            .put_to_frame(param.clone(), args[index].clone());
                    } else {
                        self.call_stack
                            .put_to_frame(param.clone(), Value::RawValue(RawValue::Nothing));
                    }
                }

                // generate vm states for function execution
                self.bytecode = bytecode.clone();
                self.pc = 0;
                self.operand_stack = vec![];

                let function_exec_result = self.run_bytecode(debug).await;

                // release frame after function execution
                let frame = self.call_stack.pop();
                if let Some(f) = frame {
                    for (_, value) in f.symbols {
                        if let Value::Handle(h) = value {
                            let _ = self.memory.release(&h);
                        }
                    }
                }

                // release any handles still on the operand stack that were never consumed
                // (e.g. return values of calls whose results weren't stored)
                let leftover_stack = std::mem::take(&mut self.operand_stack);
                for sv in leftover_stack {
                    if let Value::Handle(h) = &sv.value {
                        let _ = self.memory.release(h);
                    }
                }

                // NOTE: args do NOT need an explicit release here.
                // Each arg handle was pushed via push_to_stack (retain), then transferred
                // via get_stack_values + put_to_frame into the frame. The frame pop above
                // already released that exact retain — releasing again would be a double-free.

                // recover vm state
                self.pc = return_pc;
                self.bytecode = main_bytecode;
                self.operand_stack = prev_operand_stack;

                function_exec_result
            }
            Engine::Native(native) => {
                if args.len() < func.parameters.len() {
                    let error = VMErrorType::TypeError(TypeError::InvalidFunctionCall {
                        function: func.identifier.clone(),
                        expected: func.parameters.len() as u32,
                        received: args.len() as u32,
                    });
                    return VMExecutionResult::terminate_with_errors(error, self);
                }
                let execution_result = native(self, caller, args.clone(), debug);
                // native functions receive a copy of args; release the push retains
                for arg in &args {
                    if let Value::Handle(h) = arg {
                        let _ = self.memory.release(h);
                    }
                }
                if let Ok(result) = execution_result {
                    // native functions alloc their results with rc=0; retain once
                    // so the CALL handler's direct push keeps a consistent rc=1
                    if let Value::Handle(h) = &result {
                        let _ = self.memory.retain(h);
                    }
                    VMExecutionResult { error: None, result: Some(result) }
                } else {
                    VMExecutionResult { error: Some(execution_result.unwrap_err()), result: None }
                }
            }
            Engine::NativeAsync(async_native) => {
                if args.len() < func.parameters.len() {
                    let error = VMErrorType::TypeError(TypeError::InvalidFunctionCall {
                        function: func.identifier.clone(),
                        expected: func.parameters.len() as u32,
                        received: args.len() as u32,
                    });
                    return VMExecutionResult::terminate_with_errors(error, self);
                }
                let execution_result = async_native(self, caller, args.clone(), debug).await;
                for arg in &args {
                    if let Value::Handle(h) = arg {
                        let _ = self.memory.release(h);
                    }
                }
                if let Ok(result) = execution_result {
                    if let Value::Handle(h) = &result {
                        let _ = self.memory.retain(h);
                    }
                    VMExecutionResult { error: None, result: Some(result) }
                } else {
                    VMExecutionResult { error: Some(execution_result.unwrap_err()), result: None }
                }
            }
        };

        return execution_result;
    }

    // REFACTOR: this function should return a Result<(DataType, Vec<u8>), VMError>
    fn get_value_length(&mut self) -> (DataType, Vec<u8>) {
        let data_type = DataType::to_opcode(self.bytecode[self.pc]);
        let value_length = match data_type {
            DataType::I32 => 4,
            DataType::I64 => 8,
            DataType::U32 => 4,
            DataType::U64 => 8,
            DataType::F64 => 8,
            DataType::Nothing => 0,
            DataType::Bool => 1,
            DataType::Utf8 => {
                self.pc += 1;
                let (data_type, value) = self.get_value_length();
                if data_type != DataType::U32 {
                    panic!("bad utf8 value length")
                }

                let (string_length, _) = self.bytes_to_data(&DataType::U32, &value);
                if let Value::RawValue(RawValue::U32(val)) = string_length {
                    val.value as usize
                } else {
                    panic!("Unexpected value type for string length");
                }
            }
            DataType::StructLiteral => 4, // fields count
            DataType::Vector => 4,        // elements count
            DataType::Lambda => {
                // 4 params count, 4 function block length
                let params = 4;
                let block_offset = 4;
                let offset = Vm::read_offset(&self.bytecode[self.pc + 5..self.pc + 9]);

                (params + block_offset + offset) as usize
            }
            _ => {
                println!("data_type: {:#?}", data_type);
                panic!("Unsupported datatype")
            }
        };

        if (self.pc + value_length) >= self.bytecode.len() {
            panic!("Invalid value size at position {}", self.pc + 1);
        };

        let value_bytes = self.bytecode[self.pc + 1..self.pc + 1 + value_length].to_vec();
        self.pc += value_length;

        (data_type, value_bytes)
    }

    // REFACTOR: this function should return a Result<(Value, String), VMError>
    pub fn bytes_to_data(&mut self, data_type: &DataType, value: &Vec<u8>) -> (Value, String) {
        let printable_value;
        let value = match data_type {
            DataType::I32 => {
                let value = i32::from_le_bytes(
                    value
                        .as_slice()
                        .try_into()
                        .expect("Provided value is incorrect"),
                );
                printable_value = value.to_string();
                Value::RawValue(RawValue::I32(I32::new(value)))
            }
            DataType::I64 => {
                let value = i64::from_le_bytes(
                    value
                        .as_slice()
                        .try_into()
                        .expect("Provided value is incorrect"),
                );
                printable_value = value.to_string();
                Value::RawValue(RawValue::I64(I64::new(value)))
            }
            DataType::U32 => {
                let value = u32::from_le_bytes(
                    value
                        .as_slice()
                        .try_into()
                        .expect("Provided value is incorrect"),
                );
                printable_value = value.to_string();
                Value::RawValue(RawValue::U32(U32::new(value)))
            }
            DataType::U64 => {
                let value = u64::from_le_bytes(
                    value
                        .as_slice()
                        .try_into()
                        .expect("Provided value is incorrect"),
                );
                printable_value = value.to_string();
                Value::RawValue(RawValue::U64(U64::new(value)))
            }
            DataType::F64 => {
                let value = f64::from_le_bytes(
                    value
                        .as_slice()
                        .try_into()
                        .expect("Provided value is incorrect"),
                );
                printable_value = value.to_string();
                Value::RawValue(RawValue::F64(F64::new(value)))
            }
            DataType::Utf8 => {
                let value =
                    String::from_utf8(value.clone()).expect("Provided value is not valid UTF-8");
                printable_value = value.to_string();

                let string_obj = SelfString::new(value, self);
                let value_handle = self.memory.alloc(MemObject::String(string_obj));
                Value::Handle(value_handle)
            }
            DataType::Vector => {
                let elements_count_bytes = if value.len() >= 4 {
                    &value[value.len() - 4..]
                } else {
                    panic!("Struct literal must contain more than 4 bytes");
                };

                let elements_count = u32::from_le_bytes(
                    elements_count_bytes
                        .try_into()
                        .expect("Provided value is incorrect"),
                );
                let elements = self.get_stack_values(&elements_count);

                let mut vector = Vector::new(elements);
                vector::init_vector_members(&mut vector, &self);
                printable_value = vector.to_string(self);

                let value_handle = self.memory.alloc(MemObject::Vector(vector));
                Value::Handle(value_handle)
            }
            DataType::StructLiteral => {
                let struct_type = self.get_stack_values(&1)[0].clone();
                let fields_count_bytes = if value.len() >= 4 {
                    &value[value.len() - 4..]
                } else {
                    panic!("Struct literal must contain more than 4 bytes");
                };

                let fields_count = u32::from_le_bytes(
                    fields_count_bytes
                        .try_into()
                        .expect("Provided value is incorrect"),
                );

                // we made *2 because, we're storing the field_value and the field_name
                let mut fields: HashMap<String, Value> = HashMap::new();
                let flat_fields = self.get_stack_values(&(fields_count * 2));
                for i in (0..fields_count * 2).step_by(2) {
                    let field_name_handle = flat_fields[i as usize].clone();
                    let field_value = flat_fields[(i + 1) as usize].clone();

                    // this is because we're using the existent infra for utf8 values
                    // and they are a heap allocated value, but there is also infra to
                    // storing strings in the stack and not in the heap
                    if let Value::Handle(field_handle) = field_name_handle {
                        let field_name = self.memory.free(&field_handle);
                        if let MemObject::String(field_name) = field_name {
                            // add field with it's value to StructLiteral fields
                            fields.insert(field_name.to_string(), field_value);
                        } else {
                            // TODO: handle with self-vm errors system
                            panic!("struct field_name must be a MemObject of type string");
                        }
                    } else {
                        // TODO: handle with self-vm errors system
                        panic!("struct field_name must be a Handle of a string");
                    };
                }

                let resolved_struct_type = struct_type.as_mem_obj(self).unwrap();
                printable_value = resolved_struct_type.to_string(self);

                // here we should check if the struct exists and the each field
                // before allocating it in the heap
                let struct_literal =
                    StructLiteral::new(resolved_struct_type.to_string(self), fields, self);
                let value_handle = self.memory.alloc(MemObject::StructLiteral(struct_literal));
                Value::Handle(value_handle)
            }
            DataType::Lambda => {
                // params count
                let params_count_bytes = if value.len() >= 4 {
                    &value[0..4]
                } else {
                    panic!("lambda must contain more than 4 bytes");
                };
                let params_count = Vm::read_offset(params_count_bytes);
                let params = self.get_stack_values(&(params_count as u32));
                let params_names: Vec<String> = params
                    .iter()
                    .map(|p| p.as_string_obj(self).unwrap())
                    .collect();

                // lambda block
                let block_length_bytes = if value.len() >= 8 {
                    &value[4..8]
                } else {
                    panic!("lambda must contain more than 8 bytes");
                };
                let block_length = Vm::read_offset(block_length_bytes);

                let block_bytes = if value.len() == (8 + block_length) as usize {
                    &value[8..(8 + block_length) as usize]
                } else {
                    panic!("error. lambda block size is less than it's length offset");
                };

                let lambda_fn = MemObject::Function(Function::new(
                    "lambda".to_string(),
                    params_names,
                    Engine::Bytecode(block_bytes.to_vec()),
                ));
                let func_handle = self.memory.alloc(lambda_fn);
                printable_value = "lambda".to_string();
                Value::Handle(func_handle)
            }
            DataType::Bool => {
                if value.len() > 1 {
                    panic!("Bad boolean value")
                }

                let value = if value[0] == 0x00 {
                    printable_value = "false".to_string();
                    false
                } else {
                    printable_value = "true".to_string();
                    true
                };
                Value::RawValue(RawValue::Bool(Bool::new(value)))
            }
            DataType::Nothing => {
                printable_value = "nothing".to_string();
                Value::RawValue(RawValue::Nothing)
            }
            _ => {
                panic!("Unsupported type to get data from")
            }
        };

        (value, printable_value)
    }

    fn value_to_string(&mut self, value: Value) -> Result<String, VMErrorType> {
        Ok(value.to_string(self))
    }

    fn values_to_string(&mut self, args: Vec<Value>) -> Result<Vec<String>, VMErrorType> {
        let mut resolved_args = Vec::new();
        for val in args {
            match self.value_to_string(val) {
                Ok(v) => resolved_args.push(v),
                Err(e) => return Err(e),
            }
        }

        Ok(resolved_args)
    }

    pub fn read_offset(bytes: &[u8]) -> i32 {
        // TODO: use self-vm errors
        let arr: [u8; 4] = bytes.try_into().expect("slice with incorrect length");
        i32::from_le_bytes(arr)
    }

    fn get_function_call_args(&mut self) -> Vec<Value> {
        // get u32 value. 4 bytes based on the type plus the current
        let value_length = 3;
        if self.pc + value_length >= self.bytecode.len() {
            panic!("Invalid instruction at position {}", self.pc);
        }

        let value_bytes = &self.bytecode[self.pc..self.pc + 4];
        let number_of_args =
            u32::from_le_bytes(value_bytes.try_into().expect("Provided value is incorrect"));
        self.pc += 4; // 4 => 3 + 1 extra to leave the pc in the next opcode

        // execution
        let args = self.get_stack_values(&number_of_args);
        args
    }

    pub fn get_stack_values(&mut self, num_of_values: &u32) -> Vec<Value> {
        let mut args = Vec::with_capacity(*num_of_values as usize);

        for _ in 0..*num_of_values {
            match self.operand_stack.pop() {
                Some(v) => args.push(v.value),
                None => {
                    panic!("Stack underflow: trying to get '{num_of_values}' values from the stack")
                }
            }
        }

        args.reverse(); // invocation order
        args
    }

    /// Unwraps a Value, extracting the underlying value from BoundAccess if present.
    /// This allows property accesses to be used in binary expressions.
    fn unwrap_bound_access(&self, value: Value) -> Value {
        match value {
            Value::BoundAccess(bound) => {
                // Recursively unwrap in case of nested bound accesses
                self.unwrap_bound_access(*bound.property)
            }
            other => other,
        }
    }

    // methods for builtin handlers like vector methods
    pub fn get_handler(&self, handler: &str) -> Option<Handle> {
        self.handlers.get(handler).cloned()
    }

    pub fn add_handler(
        &mut self,
        handler_name: String,
        handle_obj: Handle,
    ) -> Result<(), VMErrorType> {
        self.handlers.insert(handler_name, handle_obj);
        Ok(())
    }

    pub fn push_to_stack(&mut self, value: Value, origin: Option<String>) {
        // The operand stack is an owner: retain any handle being pushed
        if let Value::Handle(h) = &value {
            let _ = self.memory.retain(h);
        }
        self.operand_stack.push(OperandsStackValue { value, origin });
    }

    /// Pop a value that is being *consumed* (binary ops, conditions, print args).
    /// Releases the handle retain that push_to_stack acquired.
    fn pop_from_stack(&mut self) -> Option<OperandsStackValue> {
        let sv = self.operand_stack.pop();
        if let Some(ref v) = sv {
            if let Value::Handle(h) = &v.value {
                let _ = self.memory.release(h);
            }
        }
        sv
    }

    /// Release any Handle values in a slice (used after native function calls
    /// that receive their args via get_stack_values without an owning frame).
    fn release_values(&mut self, values: &[Value]) {
        for val in values {
            if let Value::Handle(h) = val {
                let _ = self.memory.release(h);
            }
        }
    }

    // events queue methods
    pub async fn drain_events(&mut self) {
        match self.events_queue.try_recv() {
            Ok(event) => match event {
                Event::Call(f) => {
                    self.run_function(&f, None, vec![], false).await;
                }
                _ => {
                    println!("unknown event");
                }
            },
            _ => (),
        };
    }

    pub fn get_vm_notifier(&self) -> mpsc::UnboundedSender<Event> {
        self.events_sender.clone()
    }

    pub fn debug_bytecode(&mut self) {
        println!("\n--- BYTECODE ----------\n");
        for (index, byte) in self.bytecode.iter().enumerate() {
            println!("[{index}] {}", byte)
        }
        // -------
        // THIS CODE IS COMMENTED FOR THE REASON THAT
        // I DON'T KNOW HOW TO HANDLE THE BYTECODE
        // TRANSLATION WITHOUT AFFECTING THE CREATIVE
        // FL0W. SO FOR THE MOMENT WE'RE AVOIDING THE
        // PROBLEM BY COMMENTING IT.
        // ✱
        // -------
        // let mut pc = 0;
        // let mut target_pc = 0;

        // let string_offset = self.bytecode.len().to_string();
        // while pc < self.bytecode.len() {
        //     let index = (pc + 1).to_string();
        //     let mut counter = 0;
        //     let printable_index = string_offset
        //         .chars()
        //         .map(|_| {
        //             let mut result = "".to_string();
        //             if let Some(char) = index.chars().nth(counter) {
        //                 result = char.to_string();
        //             } else {
        //                 result = " ".to_string();
        //             }
        //             counter += 1;
        //             return result;
        //         })
        //         .collect::<String>();

        //     if pc >= target_pc {
        //         // print instruction
        //         let (instruction, offset) = Translator::get_instruction(pc, &self.bytecode);
        //         let raw_instruction = format!("{}|    {:#?}", printable_index, self.bytecode[pc]);
        //         println!("{} <---- {}", raw_instruction, instruction.get_type());

        //         let instruction_info = Translator::get_instruction_info(&instruction);
        //         if instruction_info.len() > 0 {
        //             println!("------------ \n{}\n------------", instruction_info);
        //         }
        //         // + 1  the normal iteration increment over the bytecode
        //         target_pc = pc + offset + 1;
        //     } else {
        //         // print bytecode index
        //         println!("{}|    {:#?}", printable_index, self.bytecode[pc]);
        //     }

        //     pc += 1;
        // }
        //println!("\n--- BYTECODE INSTRUCTIONS ----------\n");
        //println!("{:#?}", Translator::new(self.bytecode.clone()).translate());
    }

    /// Release all remaining handles held by the call stack frames, operand
    /// stack, and built-in handler table.  Call this after program execution
    /// to get a clean baseline for --memcheck: zero live handles = no leaks.
    pub fn cleanup(&mut self) {
        // 1. Drain operand stack
        let remaining_stack = std::mem::take(&mut self.operand_stack);
        for sv in remaining_stack {
            if let Value::Handle(h) = &sv.value {
                let _ = self.memory.release(h);
            }
        }

        // 2. Pop every call-stack frame (global frame included) and release handles
        while let Some(frame) = self.call_stack.pop() {
            for (_, value) in frame.symbols {
                if let Value::Handle(h) = value {
                    let _ = self.memory.release(&h);
                }
            }
        }

        // 3. Release built-in method handlers (.map, .len, .split, etc.)
        //    These are allocated with rc=0 and live only in self.handlers.
        let handlers = std::mem::take(&mut self.handlers);
        for (_, h) in handlers {
            let _ = self.memory.release(&h);
        }

        // 4. Force-free anything that remains: nested handles in Vectors,
        //    StructLiteral fields, etc. — objects that were never individually
        //    retained/released because their container was their sole owner.
        self.memory.drain_all();
    }

    pub fn memory_stats(&self) -> crate::memory::MemoryStats {
        self.memory.stats()
    }
}
