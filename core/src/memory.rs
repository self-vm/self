use std::collections::HashMap;

use crate::{
    core::error::{self, memory_errors::MemoryError, VMError, VMErrorType},
    heap::{Heap, HeapRef},
    types::object::{
        func::Function,
        native_struct::NativeStruct,
        string::SelfString,
        structs::{StructDeclaration, StructLiteral},
        vector::Vector,
    },
    vm::Vm,
};

#[derive(Debug)]
pub struct MemoryManager {
    heap: Heap,
    table: HashMap<u32, Entry>,
    next_pointer: u32,
}

impl MemoryManager {
    pub fn new() -> MemoryManager {
        MemoryManager {
            heap: Heap::new(),
            table: HashMap::new(),
            next_pointer: 0,
        }
    }

    pub fn alloc(&mut self, obj: MemObject) -> Handle {
        match obj {
            MemObject::String(_)
            | MemObject::Function(_)
            | MemObject::NativeStruct(_)
            | MemObject::StructDeclaration(_)
            | MemObject::StructLiteral(_)
            | MemObject::Vector(_) => {
                let heap_ref = self.heap.allocate(obj);
                self.gen_handle(PointerType::HeapPointer(heap_ref))
            }
        }
    }

    pub fn free(&mut self, handle: &Handle) -> MemObject {
        // free_handle removes the entry from the table and returns the pointer
        let heap_ref = self.free_handle(handle).1.as_heap_pointer();
        self.heap
            .free(heap_ref)
            .unwrap_or_else(|| panic!("handle pointer does not exist in heap"))
    }

    pub fn resolve(&self, handle: &Handle) -> &MemObject {
        let real_pointer = self.table.get(&handle.pointer);
        if let Some(rp) = real_pointer {
            match &rp.ptr {
                PointerType::HeapPointer(p) => match self.heap.get(p.clone()) {
                    Some(v) => v,
                    None => panic!("handle pointer does not exist in memory table"),
                },
            }
        } else {
            panic!("handle pointer does not exist in memory table")
        }
    }

    pub fn resolve_mut(&mut self, handle: &Handle) -> &mut MemObject {
        let real_pointer = self.table.get(&handle.pointer);
        if let Some(rp) = real_pointer {
            match &rp.ptr {
                PointerType::HeapPointer(p) => match self.heap.get_mut(p.clone()) {
                    Some(v) => v,
                    None => panic!("handle pointer does not exist in memory table"),
                },
            }
        } else {
            panic!("handle pointer does not exist in memory table")
        }
    }

    pub fn retain(&mut self, handle: &Handle) -> Result<(), VMErrorType> {
        let real_pointer = self.table.get_mut(&handle.pointer);
        if let Some(rp) = real_pointer {
            rp.rc_increment();
            Ok(())
        } else {
            Err(VMErrorType::Memory(MemoryError::InvalidHandle(
                handle.pointer,
            )))
        }
    }

    pub fn release(&mut self, handle: &Handle) -> Result<(), VMErrorType> {
        // Separate the borrow of table from the potential call to free()
        let should_free = match self.table.get_mut(&handle.pointer) {
            Some(rp) => rp.rc_decrement() < 1,
            None => {
                return Err(VMErrorType::Memory(MemoryError::InvalidHandle(
                    handle.pointer,
                )))
            }
        };

        if should_free {
            // free() calls free_handle() which removes the entry from the table
            self.free(handle);
        }

        Ok(())
    }

    pub fn stats(&self) -> MemoryStats {
        let live_handles = self.table.len();
        let live_heap = self.heap.len();
        let total_allocs = self.next_pointer as usize;

        // breakdown: count by object type
        let mut strings = 0usize;
        let mut functions = 0usize;
        let mut structs = 0usize;
        let mut vectors = 0usize;
        let mut other = 0usize;
        for entry in self.table.values() {
            if let PointerType::HeapPointer(ref hr) = entry.ptr {
                match self.heap.get(hr.clone()) {
                    Some(MemObject::String(_)) => strings += 1,
                    Some(MemObject::Function(_)) => functions += 1,
                    Some(MemObject::StructDeclaration(_))
                    | Some(MemObject::StructLiteral(_))
                    | Some(MemObject::NativeStruct(_)) => structs += 1,
                    Some(MemObject::Vector(_)) => vectors += 1,
                    _ => other += 1,
                }
            }
        }

        MemoryStats {
            live_handles,
            live_heap,
            total_allocs,
            strings,
            functions,
            structs,
            vectors,
            other,
        }
    }

    /// Force-free every object still in the heap without RC checks.
    /// Used at program shutdown to clean up nested handles (e.g. Vector
    /// elements, StructLiteral fields) that have rc=0 but were never
    /// explicitly released because the container was their only owner.
    pub fn drain_all(&mut self) {
        let remaining: Vec<Handle> = self
            .table
            .keys()
            .map(|&p| Handle::new(p))
            .collect();
        for h in remaining {
            // free_handle may panic if the handle is already gone;
            // use remove directly to be safe.
            self.table.remove(&h.pointer);
        }
        // Clear the heap too
        self.heap.drain();
    }

    fn gen_handle(&mut self, pointer: PointerType) -> Handle {
        let generated_pointer = self.next_pointer;
        self.next_pointer += 1;
        let handle = Handle::new(generated_pointer);
        self.table.insert(generated_pointer, Entry::new(pointer));
        handle
    }

    fn free_handle(&mut self, handle: &Handle) -> (u32, PointerType) {
        let val = self.table.remove(&handle.pointer);
        if val.is_none() {
            panic!("unset pointer exception")
        }

        (handle.pointer, val.unwrap().ptr.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Handle {
    pub pointer: u32,
}

impl Handle {
    pub fn new(handle_pointer: u32) -> Handle {
        Handle {
            pointer: handle_pointer,
        }
    }

    pub fn to_string(&self) -> String {
        self.pointer.to_string()
    }
}

#[derive(Debug, Clone)]
struct Entry {
    ptr: PointerType,
    rc: u32,
}

impl Entry {
    pub fn new(ptr: PointerType) -> Entry {
        Entry { ptr, rc: 0 }
    }
    pub fn rc_increment(&mut self) {
        self.rc += 1;
    }
    pub fn rc_decrement(&mut self) -> u32 {
        self.rc = self.rc.saturating_sub(1);
        self.rc
    }
}

#[derive(Clone, Debug)]
pub enum PointerType {
    HeapPointer(HeapRef),
}

impl PointerType {
    pub fn as_heap_pointer(&self) -> HeapRef {
        match self {
            PointerType::HeapPointer(v) => v.clone(),
            _ => panic!("invalid parse on PointerType as_heap_pointer method"),
        }
    }
}

// ─── Memory diagnostics ──────────────────────────────────────────────────────

#[derive(Debug)]
pub struct MemoryStats {
    pub live_handles: usize,
    pub live_heap: usize,
    pub total_allocs: usize,
    pub strings: usize,
    pub functions: usize,
    pub structs: usize,
    pub vectors: usize,
    pub other: usize,
}

impl MemoryStats {
    pub fn print(&self) {
        println!("\n╔══════════════════════════════╗");
        println!("║       MEMORY STATS           ║");
        println!("╠══════════════════════════════╣");
        println!("║  total allocs  : {:>10}  ║", self.total_allocs);
        println!("║  live handles  : {:>10}  ║", self.live_handles);
        println!("║  live heap objs: {:>10}  ║", self.live_heap);
        println!("╠══════════════════════════════╣");
        println!("║  strings       : {:>10}  ║", self.strings);
        println!("║  functions     : {:>10}  ║", self.functions);
        println!("║  structs       : {:>10}  ║", self.structs);
        println!("║  vectors       : {:>10}  ║", self.vectors);
        if self.other > 0 {
            println!("║  other         : {:>10}  ║", self.other);
        }
        println!("╠══════════════════════════════╣");
        let leaked = self.live_handles;
        if leaked == 0 {
            println!("║  leaks         :          0  ║");
            println!("║  ✓ no leaks detected         ║");
        } else {
            println!("║  leaks         : {:>10}  ║", leaked);
            println!("║  ✗ LEAKED HANDLES DETECTED   ║");
        }
        println!("╚══════════════════════════════╝");
    }
}

// STORED OBJECTS ON THE MEMORY
#[derive(Debug)]
pub enum MemObject {
    String(SelfString),
    Function(Function),
    StructDeclaration(StructDeclaration),
    StructLiteral(StructLiteral),
    NativeStruct(NativeStruct),
    Vector(Vector),
}

impl MemObject {
    pub fn llm_serialize(&self, vm: &Vm) -> String {
        match self {
            MemObject::String(x) => x.to_string(),
            MemObject::Function(x) => x.to_string(),
            MemObject::StructDeclaration(x) => x.to_string(),
            MemObject::StructLiteral(x) => x.to_string(vm),
            MemObject::NativeStruct(x) => x.serialize(vm),
            MemObject::Vector(x) => x.to_string(vm),
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        match self {
            MemObject::String(x) => x.to_string(),
            MemObject::Function(x) => x.to_string(),
            MemObject::StructDeclaration(x) => x.to_string(),
            MemObject::StructLiteral(x) => x.to_string(vm),
            MemObject::NativeStruct(x) => x.to_string(vm),
            MemObject::Vector(x) => x.to_string(vm),
        }
    }

    pub fn get_type(&self) -> String {
        match self {
            MemObject::String(_) => "string".to_string(),
            MemObject::Function(_) => "function".to_string(),
            MemObject::StructDeclaration(_) => "struct_declaration".to_string(),
            MemObject::StructLiteral(_) => "struct_literal".to_string(),
            MemObject::NativeStruct(_) => "native_struct".to_string(),
            MemObject::Vector(_) => "vector".to_string(),
        }
    }

    pub fn as_struct_declaration(&self, vm: &Vm) -> Result<StructDeclaration, VMError> {
        match self {
            MemObject::StructDeclaration(x) => Ok(x.clone()),
            _ => Err(error::throw(
                error::VMErrorType::TypeMismatch {
                    expected: "struct_declaration".to_string(),
                    received: self.get_type(),
                },
                vm,
            )),
        }
    }

    pub fn as_native_struct(&self, vm: &Vm) -> Result<&NativeStruct, VMError> {
        match self {
            MemObject::NativeStruct(x) => Ok(x),
            _ => Err(error::throw(
                error::VMErrorType::TypeMismatch {
                    expected: "NativeStruct".to_string(),
                    received: self.get_type(),
                },
                vm,
            )),
        }
    }

    pub fn as_struct_literal(&self, vm: &Vm) -> Result<StructLiteral, VMError> {
        match self {
            MemObject::StructLiteral(x) => Ok(x.clone()),
            _ => Err(error::throw(
                error::VMErrorType::TypeMismatch {
                    expected: "struct_literal".to_string(),
                    received: self.get_type(),
                },
                vm,
            )),
        }
    }
}
