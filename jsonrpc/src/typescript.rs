use std::io;
use std::path::Path;
use typescript_type_def::{
    type_expr::TypeInfo, write_definition_file, DefinitionFileOptions, TypeDef,
};

pub fn typedef_to_expr_string<T: TypeDef>(root_namespace: &Option<&str>) -> io::Result<String> {
    let mut options = DefinitionFileOptions::default();
    options.root_namespace = root_namespace.as_deref();
    let mut expr = vec![];
    <T as TypeDef>::INFO.emit_expr(&mut expr, &options)?;
    Ok(String::from_utf8(expr).unwrap())
}

pub fn export_types_to_file<T: TypeDef>(
    path: &Path,
    options: Option<DefinitionFileOptions>,
) -> io::Result<()> {
    let options = options.unwrap_or_else(|| {
        let mut options = DefinitionFileOptions::default();
        options.root_namespace = None;
        options
    });
    let mut file = std::fs::File::create(&path)?;
    write_definition_file::<_, T>(&mut file, options.clone())?;
    Ok(())
}

pub struct Method {
    pub is_notification: bool,
    pub is_positional: bool,
    pub ts_name: String,
    pub rpc_name: String,
    pub args: Vec<(String, &'static TypeInfo)>,
    pub output: Option<&'static TypeInfo>,
}

impl Method {
    pub fn new(
        ts_name: impl ToString,
        rpc_name: impl ToString,
        args: Vec<(String, &'static TypeInfo)>,
        output: Option<&'static TypeInfo>,
        is_notification: bool,
        is_positional: bool,
    ) -> Self {
        Self {
            ts_name: ts_name.to_string(),
            rpc_name: rpc_name.to_string(),
            args,
            output,
            is_notification,
            is_positional,
        }
    }

    pub fn to_string(&self, root_namespace: &Option<&str>) -> String {
        // let maybe_async = if !self.is_notification { "async " } else { "" };
        let (args, call) = if !self.is_positional {
            if let Some((name, ty)) = self.args.get(0) {
                (
                    format!("{}: {}", name, type_to_expr(ty, &root_namespace)),
                    name.to_string(),
                )
            } else {
                ("".to_string(), "undefined".to_string())
            }
            // let arg = self.args.get(0).map_or_else(
            //     || "void".to_string(),
            //     |(_name, arg)| type_to_expr(arg, &root_namespace),
            // );
            // let args = format!("{}: {}", "params", arg);
            // (args, "params".to_string())
        } else {
            let args = self
                .args
                .iter()
                .map(|(name, arg)| format!("{}: {}", name, type_to_expr(&arg, &root_namespace)))
                .collect::<Vec<String>>()
                .join(", ");
            let call = format!(
                "[{}]",
                self.args
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            (args, call)
        };
        let output = self.output.map_or_else(
            || "void".to_string(),
            |output| type_to_expr(&output, &root_namespace),
        );
        let (output, inner_method) = if !self.is_notification {
            (format!("Promise<{}>", output), "_request")
        } else {
            (output, "_notification")
        };
        format!(
            "    public {}({}): {} {{\n        return (this.{}('{}', {} as RPC.Params)) as {};\n    }}\n",
            self.ts_name, args, output, inner_method, self.rpc_name, call, output
        )
    }
}

fn type_to_expr(ty: &'static TypeInfo, root_namespace: &Option<&str>) -> String {
    let mut options = DefinitionFileOptions::default();
    options.root_namespace = root_namespace.as_deref();
    let mut expr = vec![];
    ty.emit_expr(&mut expr, &options).unwrap();
    String::from_utf8(expr).unwrap()
}
