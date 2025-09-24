pub fn infer_prompt(request: &String, context: &String) -> String {
    return format!(
        "
Analyze the query and respond with a single value in the following json format:

{{
  \"value\": response-value
}}

You are provided two elements:

query: a string that describes a condition for example:
   '<arg> is greater than 10'

context: a dictionary of variables and their current values, for example:
   {{ 'arg': 11 }}

Context variables appears in the query enclosed in < >, and you must evaluate them correctly.

Response rules: 

* For boolean or logical values use true or false.
* If the conditional expression is not met, respond with nothing.
* If there are no conditionals but you can infer the type and value, do so.
* If you cannot determine a type with certainty, respond with nothing.
* Never respond with any additional text. Only the final value.

Infer the following input: 

query: {} 
context: {{ 'arg': {} }}
",
        request.to_string(),
        context.to_string()
    );
}

pub fn resolve_prompt(query: &String) -> String {
    return format!(
        "
Respond to the following query by resolving it to a single value.

Return the result strictly in the following JSON format:

{{
  \"value\": response-value
}}

Rules:
* The response must always contain a single value that directly answers the query.
* For text values, wrap them in quotes.
* For numbers, use numeric literals.
* For boolean answers, use true or false.
* Do not include explanations, additional text, or multiple values.
* If the query cannot be reasonably resolved, respond with nothing.

Query: {}
",
        query.to_string()
    );
}

pub fn do_prompt(stdlib_defs: Vec<String>, request: &String) -> String {
    return format!(
        "You are a virtual machine assistant with access to the following native modules:\n\n{}\n\n
        
You must respond to the following instruction with a list of JSON objects, where each object contains:

- 'module': the name of the module from the list above,
- 'member': the specific function name to call (from the members),
- 'params': an array of arguments. If you cannot infer the parameter or the parameter value is dynamic set to {{self_runtime}} string value

You must only use the modules and members listed above. Do not invent anything.

Respond only with JSON. Do not include any explanations or markdown.

Instruction: {}",
        stdlib_defs.join("\n\n"),
        request
    );
}

pub fn act_chain_prompt(stdlib_defs: Vec<String>, purpose: &String, end_condition: &String) -> String {
    return format!(
        "
You are a virtual machine orchestrator that given a purpose and an end condition will act with a chain of thoughts using the following native modules until the end condition mets. You have access to the following native modules to act:

    {}

You must answer a json with the following structure:

{{
   \"link_def\": \"an abstract defining the current link in one sentence\", 
   \"link_action\": {{ // current link of the whole thoughts chain
         - 'module': the name of the module from the list above,
         - 'member': the specific function name to call (from the members),
         - 'params': an array of arguments. If you cannot infer the parameter or the parameter value is dynamic set to {{self_runtime}} string value   
    }}
    \"next_links\": [
         \"\", \"\", \"\" // strings defining an abstract of what should be the next step, but only two steps ahead.  
    ]
}}

You must only use the modules and members listed above. Do not invent anything.

Respond only with JSON. Do not include any explanations or markdown.

if the chain end condition is met, answer a json with this structure: {{ 
\"end_condition\": \"\" // the end condition you used, 
\"result\": \"\" // the description of the condition result 
\"end\": true 
    }}

Instruction: {}
End condition: {}",
        stdlib_defs.join("\n\n"),
        purpose, 
        end_condition
    );
}