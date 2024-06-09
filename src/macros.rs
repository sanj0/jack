#[macro_export]
macro_rules! class_template {
    () => {
        r#".source {source}
.class public {name}
.super {extends}

{header}

.method public <init>()V
    aload_0
    invokenonvirtual {extends}/<init>()V
{init}
    return
.end method

.method public static main([Ljava/lang/String;)V
{main}
    return
.end method
{footer}"#
    };
}

#[macro_export]
macro_rules! bool_bi_op {
    ($node:expr, $class:expr, $opcode_i:expr) => {{
        $class.append_main(&line_directive!($node)).main_endl();
        $class.append_main(opcodes::I_SUB).main_endl();
        let true_label = format!("True{}{}", $opcode_i, $class.main.len());
        let end_label = format!("End{}{}", $opcode_i, $class.main.len());
        $class
            .push_main($opcode_i)
            .append_main(&true_label)
            .main_endl();
        $class.append_main(opcodes::ICONST_0).main_endl();
        $class
            .push_main(opcodes::GOTO)
            .append_main(&end_label)
            .main_endl();
        $class.push_main(&true_label).append_main(":").main_endl();
        $class.append_main(opcodes::ICONST_1).main_endl();
        $class.push_main(&end_label).append_main(":").main_endl();
    }};
}

#[macro_export]
macro_rules! expect_type_info {
    ($node:expr) => {
        $node
            .stack
            .as_ref()
            .ok_or_else(|| CodeGenErr::NotAnalyzedErr($node.loc))?
    };
}

#[macro_export]
macro_rules! expect_var_info {
    ($node:expr) => {
        $node
            .vars
            .as_ref()
            .ok_or_else(|| CodeGenErr::NotAnalyzedErr($node.loc))?
    };
}

#[macro_export]
macro_rules! line_directive {
    ($node:expr) => {
        format!("{} {}", opcodes::DIR_LINE, $node.loc.row)
    };
}

#[macro_export]
macro_rules! invoke {
    ($node:expr, $class:expr, $invoke_opcode:expr, $name:expr, $nargs:expr, $returns:expr) => {
        $class
            .push_main($invoke_opcode)
            .append_main($name)
            .append_main("(");
        let mut args = String::new();
        let mut stack = $node.stack.as_ref().unwrap().clone();
        for _ in 0..$nargs {
            args.push_str(&stack.pop().unwrap().ty.to_opcode());
        }
        $class
            .append_main(&args)
            .append_main(")")
            .append_main($returns)
            .main_endl();
    };
    (generic: $node:expr, $class:expr, $invoke_opcode:expr, $name:expr, $nargs:expr, $returns:expr) => {
        $class
            .push_main($invoke_opcode)
            .append_main($name)
            .append_main("(");
        let mut args = String::new();
        for _ in 0..$nargs {
            args.push_str(opcodes::TYPE_OBJECT);
        }
        $class
            .append_main(&args)
            .append_main(")")
            .append_main($returns)
            .main_endl();
    };
    ($node:expr, $class:expr, $invoke_opcode:expr, $name:expr, types: $args:expr, $returns: expr) => {
        $class
            .push_main($invoke_opcode)
            .append_main($name)
            .append_main("(");
        let mut args = String::new();
        for arg in $args.iter() {
            args.push_str(arg);
        }
        $class
            .append_main(&args)
            .append_main(")")
            .append_main($returns)
            .main_endl();
    };
}

#[macro_export]
macro_rules! get_static {
    ($class:expr, $obj:expr, $type:expr) => {
        $class
            .push_main(opcodes::GET_STATIC)
            .push_main($obj)
            .append_main($type)
            .main_endl();
    };
}

#[macro_export]
macro_rules! int_to_integer {
    ($node:expr, $class:expr) => {
        invoke!(
            $node,
            $class,
            opcodes::INVOKE_STATIC,
            "java/lang/Integer/valueOf",
            types: [opcodes::TYPE_INT],
            opcodes::TYPE_INTEGER
        )
    };
}

#[macro_export]
macro_rules! integer_to_int {
    ($node:expr, $class:expr) => {{
        $class.push_main(opcodes::CHECK_CAST).append_main(opcodes::CLASS_INTEGER).main_endl();
        invoke!(
            $node,
            $class,
            opcodes::INVOKE_VIRTUAL,
            "java/lang/Integer/intValue",
            0,
            opcodes::TYPE_INT
        );
    }};
}

#[macro_export]
macro_rules! object_to_string {
    ($class:expr) => {{
        $class
            .push_main(opcodes::CHECK_CAST)
            .append_main(opcodes::CLASS_STRING)
            .main_endl();
    }}
}

#[macro_export]
macro_rules! object_to_list {
    ($class:expr) => {{
        $class
            .push_main(opcodes::CHECK_CAST)
            .append_main(opcodes::CLASS_ARRAY_LIST)
            .main_endl();
    }}
}

/// Forth's `over` operator
#[macro_export]
macro_rules! over {
    ($node:expr, $class:expr) => {
        $class.append_main(opcodes::DUP2).main_endl();
        $class.append_main(opcodes::POP).main_endl();
        $class.append_main(opcodes::SWAP).main_endl();
    };
}

#[macro_export]
macro_rules! list_len {
    ($node:expr, $class:expr) => {
        invoke!(
            $node,
            $class,
            opcodes::INVOKE_VIRTUAL,
            "java/util/ArrayList/size",
            0,
            opcodes::TYPE_INT
        );
    };
}

#[macro_export]
macro_rules! swap {
    ($class:expr) => {
        $class.append_main(opcodes::SWAP).main_endl();
    };
}

#[macro_export]
macro_rules! dup {
    ($class:expr) => {
        $class.append_main(opcodes::DUP).main_endl();
    };
}


#[macro_export]
macro_rules! new_list {
    ($node:expr, $class:expr) => {{
        $class
            .push_main(opcodes::NEW)
            .append_main(opcodes::CLASS_ARRAY_LIST)
            .main_endl();
        dup!($class);
        invoke!(
            $node,
            $class,
            opcodes::INVOKE_SPECIAL,
            "java/util/ArrayList/<init>",
            0,
            opcodes::TYPE_VOID
        );
    }}
}
