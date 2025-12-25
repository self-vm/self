/*
HERE WE DEFINE THE LOGIC OF THE AI STD MODULE.
CURRENTLY WE ARE USING THE OPENAI LLM
BUT WE COULD IN THE FUTURE IMPLEMENT ANOTHER
PROVIDER OR ENABLE USER IMPLEMENTATION OF
PROVIDER.
*/

use std::vec;

use futures::future::BoxFuture;
use serde_json::Value as SValue;

use crate::{
    core::{
        error::{
            self, action_errors::ActionError, ai_errors::AIError, type_errors::TypeError, VMError,
            VMErrorType,
        },
        logs::write_log,
    },
    memory::{Handle, MemObject},
    std::{
        ai::{
            prompts::{act_chain_prompt, do_prompt, infer_prompt, resolve_prompt},
            providers::{fetch_ai, ChatResponse},
            types::{AIAction, Action, Chain, ChainLinkJson, Link, UnfoldStore},
        },
        gen_native_modules_defs, generate_native_module, get_native_module_type,
        heap_utils::put_string,
        utils::cast_json_value,
        vector, NativeMember,
    },
    types::{
        object::{
            func::{Engine, Function},
            native_struct::NativeStruct,
            vector::Vector,
        },
        raw::{bool::Bool, f64::F64, utf8::Utf8, RawValue},
        Value,
    },
    vm::Vm,
};

fn get_response_json(response: &String) -> String {
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    cleaned.to_string()
}

fn ai_response_parser(response: &String, vm: &mut Vm) -> Option<Value> {
    let cleaned = get_response_json(response);
    let json: SValue = serde_json::from_str(cleaned.as_str()).ok()?;
    let raw_value = json.get("value")?;

    if raw_value.is_boolean() {
        let bool = raw_value.as_bool().unwrap();
        Some(Value::RawValue(RawValue::Bool(Bool::new(bool))))
    } else if raw_value.is_number() {
        let value = raw_value.as_f64().unwrap();
        Some(Value::RawValue(RawValue::F64(F64::new(value))))
    } else if raw_value.is_string() {
        let s = raw_value.as_str()?;
        if s.trim().is_empty() || s.trim() == "nothing" {
            Some(Value::RawValue(RawValue::Nothing))
        } else {
            let value = raw_value.as_str().unwrap();
            let handle = put_string(vm, value.to_string());
            Some(Value::Handle(handle))
        }
    } else {
        Some(Value::RawValue(RawValue::Nothing))
    }
}

// infer
pub fn infer_def() -> NativeMember {
    NativeMember {
        name: "infer".to_string(),
        description: "infer an output value with AI based on an input prompt".to_string(),
        params: Some(vec![
            "prompt(string)".to_string(),
            "context(string)".to_string(),
        ]),
    }
}

pub fn infer(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<Result<Value, VMError>> {
    Box::pin(async move {
        let request_ref = params[0].clone();
        let request = request_ref.as_string_obj(vm)?;
        let context_ref = params[1].clone();
        let context = context_ref.as_string_obj(vm)?;

        if debug {
            println!("AI <- {}({})", request, context.to_string());
        }

        // we should try to avoid prompt injection
        // maybe using multiple prompts?
        let prompt = infer_prompt(&request, &context);
        let res = fetch_ai(prompt).await;
        let res = match res {
            Ok(r) => r,
            Err(vm_err) => {
                return Err(error::throw(vm_err, vm));
            }
        };

        if !res.status().is_success() {
            return Err(error::throw(
                VMErrorType::AI(AIError::AIFetchError(res.status().to_string())),
                vm,
            ));
        }

        let response: ChatResponse = res.json().await.expect("AI: Failed to parse response");
        let answer = &response.choices[0].message.content;

        if debug {
            println!("AI -> {}", answer);
        }

        let parsed_answer = ai_response_parser(answer, vm);
        if let Some(v) = parsed_answer {
            return Ok(v);
        } else {
            return Ok(Value::RawValue(RawValue::Nothing));
        }
    })
}

// resolve: given a query, resolve to a value
pub fn resolve_obj() -> MemObject {
    MemObject::Function(Function::new(
        "resolve".to_string(),
        vec!["query".to_string()], // TODO: load params to native functions
        Engine::NativeAsync(resolve),
    ))
}

pub fn resolve_def() -> NativeMember {
    NativeMember {
        name: "resolve".to_string(),
        description: "resolve a query into a single output value using AI. eg: url of nike"
            .to_string(),
        params: Some(vec!["query(string)".to_string()]),
    }
}

pub fn resolve(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<Result<Value, VMError>> {
    Box::pin(async move {
        let query_ref = params[0].clone();
        let query = query_ref.as_string_obj(vm)?;

        if debug {
            println!("AI.resolve <- {}", query);
        }

        // we should try to avoid prompt injection
        // maybe using multiple prompts?
        let prompt = resolve_prompt(&query);
        let res = fetch_ai(prompt).await;
        let res = match res {
            Ok(r) => r,
            Err(vm_err) => {
                return Err(error::throw(vm_err, vm));
            }
        };

        if !res.status().is_success() {
            return Err(error::throw(
                VMErrorType::AI(AIError::AIFetchError(res.status().to_string())),
                vm,
            ));
        }

        let response: ChatResponse = res.json().await.expect("AI: Failed to parse response");
        let answer = &response.choices[0].message.content;

        if debug {
            println!("AI -> {}", answer);
        }

        let parsed_answer = ai_response_parser(answer, vm);
        if let Some(v) = parsed_answer {
            return Ok(v);
        } else {
            return Ok(Value::RawValue(RawValue::Nothing));
        }
    })
}

// do
pub fn do_fn(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<Result<Value, VMError>> {
    Box::pin(async move {
        let request_ref = params[0].clone();
        let request = request_ref.as_string_obj(vm)?;

        if debug {
            println!("AI.DO <- {}", request);
        }

        let stdlib_defs: Vec<String> = gen_native_modules_defs()
            .iter()
            .map(|nm| nm.to_string())
            .collect();

        // we should try to avoid prompt injection
        // maybe using multiple prompts?
        let prompt = do_prompt(stdlib_defs, &request);
        let res = fetch_ai(prompt).await;
        let res = match res {
            Ok(r) => r,
            Err(vm_err) => {
                return Err(error::throw(vm_err, vm));
            }
        };

        if !res.status().is_success() {
            println!("AI.DO (FAILED) -> {}", res.status());
            return Err(error::throw(
                VMErrorType::AI(AIError::AIFetchError(res.status().to_string())),
                vm,
            ));
        }

        let response: ChatResponse = res.json().await.expect("AI.DO: Failed to parse response");
        let answer = &response.choices[0].message.content;

        if debug {
            println!("AI -> {}", answer);
        }

        let cleaned = get_response_json(answer);
        let instructions: Vec<AIAction> = if let Ok(val) = serde_json::from_str(cleaned.as_str()) {
            val
        } else {
            return Ok(Value::RawValue(RawValue::Nothing));
        };
        if instructions.len() < 1 {
            return Ok(Value::RawValue(RawValue::Nothing));
        }

        // for the moment the function is allocated on
        // execution. but we should have a way of on a
        // native module import executed the generic code
        // to have things on scope, like, exec function.
        let exec_fn = Function::new("exec".to_string(), vec![], Engine::NativeAsync(exec));
        let exec_ref = vm.memory.alloc(MemObject::Function(exec_fn));

        let actions: Vec<Action> = instructions
            .iter()
            .map(|instr| {
                Action::new(
                    instr.module.clone(),
                    exec_ref.clone(),
                    instr.member.clone(),
                    instr
                        .params
                        .iter()
                        .map(|p| {
                            if let Some(v) = cast_json_value(p) {
                                v
                            } else {
                                Value::RawValue(RawValue::Nothing)
                            }
                        })
                        .collect::<Vec<Value>>(),
                )
            })
            .collect();

        if debug {
            println!("AI.DO <- {:#?}", actions)
        }

        let mut actions_ref = vec![];
        for action in actions {
            actions_ref.push(Value::Handle(
                vm.memory
                    .alloc(MemObject::NativeStruct(NativeStruct::Action(action))),
            ));
        }

        // store all actions ref in a vector and return the
        // vector allocated heap ref
        let vector = Vector::new_initialized(actions_ref, vm);
        let vector_handle = vm.memory.alloc(MemObject::Vector(vector));
        return Ok(Value::Handle(vector_handle));
    })
}

// chain
pub fn chain_obj() -> MemObject {
    MemObject::Function(Function::new(
        "chain".to_string(),
        vec![
            "purpose(string)".to_string(),
            "end_condition(string)".to_string(),
        ], // TODO: load params to native functions
        Engine::NativeAsync(chain),
    ))
}

// FLOW OF THE CHAIN GENERATION:
// chain -> generate master link -> unfold until end

pub fn chain(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<Result<Value, VMError>> {
    Box::pin(async move {
        if params.len() < 2 {
            return Err(error::throw(
                VMErrorType::TypeError(TypeError::InvalidArgsCount {
                    expected: 2,
                    received: params.len() as u32,
                }),
                vm,
            ));
        }

        let purpose_handle = params[0].clone();
        let purpose = purpose_handle.as_string_obj(vm)?;
        let end_condition_handle = params[1].clone();
        let end_condition = end_condition_handle.as_string_obj(vm)?;

        if debug {
            println!("AI.CHAIN <- {}", purpose);
        }

        // get base libs
        let stdlib_defs = get_stdlib_defs();
        let master_link = generate_link(
            &purpose,
            &end_condition,
            &vec![],
            vm,
            &stdlib_defs,
            "NORMAL MODE".to_string(),
            debug,
        )
        .await?;

        // store all actions ref in a vector and return the
        // vector allocated heap ref
        let chain = Chain::new_initialized(purpose, end_condition, vec![master_link], vm);
        let chain_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::Chain(chain)));
        return Ok(Value::Handle(chain_handle));
    })
}

pub async fn generate_link(
    purpose: &String,
    end_condition: &String,
    context: &Vec<String>,
    vm: &mut Vm,
    available_libs: &Vec<String>,
    mode: String,
    debug: bool,
) -> Result<Link, VMError> {
    // we should try to avoid prompt injection
    // maybe using multiple prompts?
    let prompt = act_chain_prompt(available_libs, &mode, purpose, end_condition, context);

    if debug {
        write_log("PROMPT", &prompt);
    }
    let res = fetch_ai(prompt).await;
    let res = match res {
        Ok(r) => r,
        Err(vm_err) => {
            return Err(error::throw(vm_err, vm));
        }
    };

    if !res.status().is_success() {
        println!("AI.CHAIN (FAILED) -> {}", res.status());
        return Err(error::throw(
            VMErrorType::AI(AIError::AIFetchError(res.status().to_string())),
            vm,
        ));
    }

    let response: ChatResponse = res
        .json()
        .await
        .expect("AI.CHAIN: Failed to parse response");
    let answer = &response.choices[0].message.content;

    if debug {
        println!("AI.CHAIN -> {}", answer);
    }

    let cleaned = get_response_json(answer);
    if debug {
        println!("AI.CHAIN [RESPONSE] -> {}", cleaned);
    }
    let chain_link: ChainLinkJson = if let Ok(val) = serde_json::from_str(cleaned.as_str()) {
        val
    } else {
        // change this to parse error
        return Err(error::throw(
            VMErrorType::AI(AIError::AIFetchError("cannot decode response".to_string())),
            vm,
        ));
    };

    if debug {
        println!("AI.CHAIN <- {:#?}", chain_link)
    }

    // todo:
    // for the moment the function is allocated on
    // execution. but we should have a way of on a
    // native module import executed the generic code
    // to have things on scope, like, exec function.
    let exec_fn = Function::new("exec".to_string(), vec![], Engine::NativeAsync(exec));
    let exec_ref = vm.memory.alloc(MemObject::Function(exec_fn));

    let link_action = Action::new(
        chain_link.link_action.module.clone(),
        exec_ref.clone(),
        chain_link.link_action.member.clone(),
        chain_link
            .link_action
            .params
            .iter()
            .map(|p| {
                if let Some(v) = cast_json_value(p) {
                    v
                } else {
                    Value::RawValue(RawValue::Nothing)
                }
            })
            .collect::<Vec<Value>>(),
    );
    return Ok(Link::new_initialized(
        chain_link.link_def,
        link_action,
        chain_link.end,
        chain_link.end_condition,
        chain_link.result,
        vm,
    ));
}

// unfold and traverse a chain links
pub fn unfold_obj() -> MemObject {
    MemObject::Function(Function::new(
        "unfold".to_string(),
        vec!["callback(action)".to_string()], // TODO: load params to native functions
        Engine::NativeAsync(unfold),
    ))
}

pub fn unfold(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<'_, Result<Value, VMError>> {
    Box::pin(async move {
        // resolve 'self'
        let (_self, _self_ref) = if let Some(_this) = _self {
            if let MemObject::NativeStruct(NativeStruct::Chain(ch)) = vm.memory.resolve(&_this) {
                (ch, _this)
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        };

        // get traverse callback
        let callback = params[0].as_function_obj(vm)?;
        if callback.parameters.len() < 1 {
            return Err(error::throw(
                VMErrorType::TypeError(TypeError::InvalidArgsCount {
                    expected: 1,
                    received: 0,
                }),
                vm,
            ));
        }

        // get chain links
        let mut links: Vec<Link> = _self
            .shape
            .property_access("links")
            .unwrap_or(Value::RawValue(RawValue::Nothing))
            .as_vector_obj(vm)?
            .elements
            .iter()
            .map(|v| v.as_native_struct(vm)?.as_link(vm))
            .collect::<Result<Vec<Link>, VMError>>()?;

        // get base libs
        let stdlib_defs = get_stdlib_defs();

        // start chain traversing, here occurs the magic
        if debug {
            println!("CHAIN.TRAVERSE <- {}", _self.to_string(vm));
        }

        // let traversing context
        let mut memory = UnfoldStore::new();
        let chain_purpose = _self
            .shape
            .property_access("purpose")
            .unwrap_or(Value::RawValue(RawValue::Nothing))
            .as_string_obj(vm)?;
        let chain_end_condition = _self
            .shape
            .property_access("end_condition")
            .unwrap_or(Value::RawValue(RawValue::Nothing))
            .as_string_obj(vm)?;

        let mut result = Value::RawValue(RawValue::Nothing);
        loop {
            // if current link is end link
            // break the loop
            let current_link = links[links.len() - 1].clone();
            // let def = current_link
            //     .shape
            //     .property_access("def")
            //     .unwrap_or(Value::RawValue(RawValue::Nothing))
            //     .as_string_obj(vm)?;
            let is_end = current_link
                .shape
                .property_access("is_end")
                .unwrap_or(Value::RawValue(RawValue::Nothing))
                .as_bool(vm)?;

            if is_end {
                let result_string = current_link
                    .shape
                    .property_access("result")
                    .unwrap_or(Value::RawValue(RawValue::Nothing))
                    .as_string_obj(vm)?;
                let handle = put_string(vm, result_string);
                result = Value::Handle(handle);
                break;
            }

            // the action is performed by the user here we only
            // check if "continue" property is true to continue
            // and generate a new link.
            let link_handle = vm.memory.alloc(MemObject::NativeStruct(NativeStruct::Link(
                links[links.len() - 1].clone(),
            )));

            // before executind the unfold callback, lets check
            // if we need to enter in <module session> mode

            let exec_result = vm
                .run_function(&callback, None, vec![Value::Handle(link_handle)], debug)
                .await;
            if let Some(err) = exec_result.error {
                return Err(err);
            }

            let conclusion = if let Some(r) = exec_result.result {
                let handle = r.as_handle()?;
                let cb_struct = vm.memory.resolve(&handle).as_struct_literal(vm)?;
                let continue_unfolding = if let Some(v) = cb_struct.property_access("continue") {
                    v.as_bool(vm)?
                } else {
                    false
                };
                let link_resolved_value = if let Some(v) = cb_struct.property_access("resolved") {
                    v
                } else {
                    Value::RawValue(RawValue::Nothing)
                };

                if !continue_unfolding {
                    // return the last resolved value on the callback
                    return Ok(link_resolved_value);
                }

                link_resolved_value
            } else {
                return Err(error::throw(
                    VMErrorType::AI(AIError::AIActionForcedAbort(
                        "unfold callback must return a boolean type. true for action exeuction false for aborting.".to_string(),
                    )),
                    vm,
                ));
            };

            // check if the chain should enter in session mode based
            // on the output of the last executed action and generate
            // the available libs members based on the mode and set
            // the session mode on the chain memory
            let (libs_defs, session_mode) = if memory.session {
                // if already in session mode
                if let Value::Handle(h) = conclusion.clone() {
                    if let MemObject::NativeStruct(ns) = vm.memory.resolve(&h) {
                        match ns.property_access("__is_session_ended") {
                            Some(v) => {
                                if let Ok(ended) = v.as_bool(vm) {
                                    if ended {
                                        memory.session = false;
                                        (&stdlib_defs, false)
                                    } else {
                                        (&memory.lib_defs.clone(), true)
                                    }
                                } else {
                                    (&memory.lib_defs.clone(), true)
                                }
                            }
                            None => (&memory.lib_defs.clone(), true),
                        }
                    } else {
                        (&memory.lib_defs.clone(), true)
                    }
                } else {
                    (&memory.lib_defs.clone(), true)
                }
            } else {
                // in normal mode
                if let Some(session) = enter_session_mode(vm, &conclusion) {
                    let (instance_name, def) = session;
                    memory.lib_defs = vec![def];
                    memory.session = true;
                    // set session handle in the callstack scope to
                    // resolve Actions calls inside the session
                    // TODO: we should remove from the frame eventually (probably)
                    vm.call_stack
                        .put_to_frame(instance_name.to_string(), conclusion.clone());
                    (&memory.lib_defs.clone(), true)
                } else {
                    memory.session = false;
                    (&stdlib_defs, false)
                }
            };

            if debug {
                println!(
                    "CHAIN MODE: {:#?}",
                    if session_mode { "SESSION" } else { "NORMAL" }
                );
            }

            // process the current link
            let current_def = current_link
                .shape
                .property_access("def")
                .unwrap_or(Value::RawValue(RawValue::Nothing))
                .as_string_obj(vm)?;

            // the context is a store during the whole execution,
            // a hashmap that has steps, and its conclusions
            // a.k.a variables of the chain
            memory.insert_entry(current_def, conclusion);
            let context = memory.context_to_string_vec(vm);

            // generate the next link
            let mut next_link = generate_link(
                &chain_purpose,
                &chain_end_condition,
                &context,
                vm,
                &libs_defs,
                if session_mode {
                    "SESSION MODE".to_string()
                } else {
                    "NORMAL MODE".to_string()
                },
                debug,
            )
            .await?;
            if debug {
                write_log(
                    "THOUGHT",
                    &next_link
                        .shape
                        .property_access("def")
                        .unwrap_or(Value::RawValue(RawValue::Nothing))
                        .as_string_obj(vm)?,
                );
            }

            // we explore the links looking for
            // runtime defined variables and resolving
            // their actual values in memory
            if let Some(a) = next_link.shape.property_access("action") {
                let mut action = a.as_native_struct(vm)?.as_action(vm)?;
                let mut resolved_args = vec![];
                for arg in action.args {
                    if let Value::RawValue(RawValue::Utf8(argv)) = &arg {
                        let argv = &argv.value;
                        if argv.starts_with("{variable_") && argv.ends_with('}') {
                            // resolve arg
                            if let Some(memory_entry) = memory.resolve(&argv[1..argv.len() - 1]) {
                                resolved_args.push(memory_entry.value.clone());
                                continue;
                            }
                        }
                    }

                    resolved_args.push(arg);
                }

                // free previous action and set the new
                action.args = resolved_args;
                let new_action_handle = vm
                    .memory
                    .alloc(MemObject::NativeStruct(NativeStruct::Action(action)));
                next_link
                    .shape
                    .property_set("action", Value::Handle(new_action_handle));

                let prev_action_handle = a.as_handle()?;
                vm.memory.free(&prev_action_handle);
            }

            links.push(next_link);
        }

        Ok(result)
    })
}

// Action type methods
pub fn exec(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<'_, Result<Value, VMError>> {
    Box::pin(async move {
        // resolve 'self'
        let (_self, _self_ref) = if let Some(_this) = _self {
            if let MemObject::NativeStruct(NativeStruct::Action(ns)) = vm.memory.resolve(&_this) {
                (ns, _this)
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        };

        if debug {
            println!("ACTION <- {}.{}", _self.module, _self.member);
        }
        let native_module_type = get_native_module_type(&_self.module);

        // if the module cannot be resolved with the stdlib modules check
        // the callstack and try to find the resolve the name and call
        // the member of the resolved value
        if let Some(native_module_type) = native_module_type {
            let native_module = generate_native_module(native_module_type);
            let fields = native_module.1;
            let member = if let Some(member) = fields.iter().find(|m| m.0 == _self.member) {
                member
            } else {
                return Err(error::throw(
                    VMErrorType::Action(ActionError::InvalidMember {
                        module: _self.module.clone(),
                        member: _self.member.clone(),
                    }),
                    vm,
                ));
            };

            match &member.1 {
                MemObject::Function(f) => {
                    let mut consumer_param_counter = 0;
                    let resolved_action_params: Vec<Value> = _self
                    .args
                    .iter()
                    .enumerate()
                    .map(|(index, arg)| match arg.as_string_obj(vm).ok().as_deref() {
                        Some(v) => {
                            if v.contains("{self_runtime}") {
                                let param = params
                                    .get(consumer_param_counter)
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        eprintln!(
                                            "action runtime defined param cannot be populated (index: {})",
                                            index
                                        );
                                        Value::RawValue(RawValue::Nothing)
                                    });
                                consumer_param_counter += 1;
                                param
                            } else {
                                arg.clone()
                            }
                        }
                        _ => arg.clone(),
                    })
                    .collect();

                    let execution = vm
                        .run_function(&f.clone(), Some(_self_ref), resolved_action_params, debug)
                        .await;
                    if let Some(err) = execution.error {
                        return Err(err);
                    }
                    if let Some(result) = execution.result {
                        return Ok(result);
                    }
                    return Ok(Value::RawValue(RawValue::Nothing));
                }
                _ => {
                    // TODO: use self-vm errors system
                    // in principle this should not happen since
                    // to the AI should arrive only valid callable
                    // members from the stdlib modules
                    panic!("error, member is not callable");
                }
            }
        } else {
            if let Some(struct_handle) = vm.call_stack.resolve(&_self.module) {
                if let Value::Handle(h) = struct_handle {
                    let resolved_struct = vm.memory.resolve(&h).as_native_struct(vm)?;
                    if let Some(member) = resolved_struct.property_access(&_self.member) {
                        let function = member.as_function_obj(vm)?;
                        let execution = vm
                            .run_function(&function, Some(h), _self.args.clone(), debug)
                            .await;
                        if let Some(err) = execution.error {
                            return Err(err);
                        }
                        if let Some(result) = execution.result {
                            return Ok(result);
                        }
                        return Ok(Value::RawValue(RawValue::Nothing));
                    }
                }
            };

            panic!("cannot resolve exec of {}", _self.module);
        }
    })
}

// utils functions
fn enter_session_mode(vm: &mut Vm, conclusion: &Value) -> Option<(String, String)> {
    let handle = match conclusion {
        Value::Handle(h) => h,
        _ => return None,
    };

    let ns = match vm.memory.resolve(&handle) {
        MemObject::NativeStruct(ns) => ns,
        _ => return None,
    };

    // if is_sessionable field exists and it's true, enter session mode
    let is_sessionable = match ns.property_access("is_sessionable") {
        Some(v) => {
            if let Ok(v) = v.as_bool(vm) {
                v
            } else {
                return None;
            }
        } // aquí validas tipo
        None => return None,
    };

    if !is_sessionable {
        return None;
    }

    // if no defs defined on the struct avoid session mode
    let instance_name = "random_generated".to_string();
    let defs = match ns.get_struct_defs(&instance_name) {
        Some(d) => d,
        None => return None,
    };
    return Some((instance_name, defs));
}

fn get_stdlib_defs() -> Vec<String> {
    gen_native_modules_defs()
        .iter()
        .map(|nm| nm.to_string())
        .collect()
}
