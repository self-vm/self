use crate::{
    memory::Handle,
    types::{
        raw::{utf8::Utf8, RawValue},
        Value,
    },
    vm::Vm,
};

#[derive(Debug)]
pub struct Action {
    pub module: String,
    pub member: String,
    pub exec: Handle, // handle to the executor function
    pub args: Vec<Value>,
}

impl Action {
    pub fn new(module: String, exec: Handle, member: String, args: Vec<Value>) -> Action {
        Action {
            module,
            exec,
            member,
            args,
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        format!(
            r#"Action({}.{}) {{
  args: [{}]
}}"#,
            self.module,
            self.member,
            self.args
                .iter()
                .map(|v| v.to_string(vm))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn property_access(&self, property: &str) -> Option<Value> {
        match property {
            "module" => Some(Value::RawValue(RawValue::Utf8(Utf8::new(
                self.module.clone(),
            )))),
            "member" => Some(Value::RawValue(RawValue::Utf8(Utf8::new(
                self.member.clone(),
            )))),
            "exec" => Some(Value::Handle(self.exec.clone())),
            //"params" => self.params,
            _ => None,
        }
    }
}
