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
    core::error::{
        self, action_errors::ActionError, ai_errors::AIError, type_errors::TypeError, VMError,
        VMErrorType,
    },
    memory::{Handle, MemObject},
    std::{
        ai::{
            prompts::{act_chain_prompt, do_prompt, infer_prompt, resolve_prompt},
            providers::{fetch_ai, ChatResponse},
            types::{AIAction, Action, Chain, ChainLinkJson, Link},
        },
        gen_native_modules_defs, generate_native_module, get_native_module_type,
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

fn ai_response_parser(response: &String) -> Option<Value> {
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
            Some(Value::RawValue(RawValue::Utf8(Utf8::new(
                value.to_string(),
            ))))
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

        let parsed_answer = ai_response_parser(answer);
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

        let parsed_answer = ai_response_parser(answer);
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
        let native_module_type = if let Some(nmt) = get_native_module_type(&_self.module) {
            nmt
        } else {
            return Err(error::throw(
                VMErrorType::Action(ActionError::InvalidModule(_self.module.clone())),
                vm,
            ));
        };
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
                        Some("{self_runtime}") => {
                            let param =
                                params
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

        let stdlib_defs: Vec<String> = gen_native_modules_defs()
            .iter()
            .map(|nm| nm.to_string())
            .collect();

        // we should try to avoid prompt injection
        // maybe using multiple prompts?
        let prompt = act_chain_prompt(stdlib_defs, &purpose, &end_condition);
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
        let chain_link: ChainLinkJson = if let Ok(val) = serde_json::from_str(cleaned.as_str()) {
            val
        } else {
            return Ok(Value::RawValue(RawValue::Nothing));
        };

        if debug {
            println!("AI.CHAIN <- {:#?}", chain_link)
        }

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
        let master_link = Link::new_initialized(chain_link.link_def, link_action, vm);

        // store all actions ref in a vector and return the
        // vector allocated heap ref
        let chain = Chain::new_initialized(purpose, end_condition, vec![master_link], vm);
        let chain_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::Chain(chain)));
        return Ok(Value::Handle(chain_handle));
    })
}
