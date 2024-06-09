//--- I_CONST_X pushed the integer constant X onto the stack ---//
pub const ICONST_M1: &str = "iconst_m1";
pub const ICONST_0: &str = "iconst_0";
pub const ICONST_1: &str = "iconst_1";
pub const ICONST_2: &str = "iconst_2";
pub const ICONST_3: &str = "iconst_3";
pub const ICONST_4: &str = "iconst_4";
pub const ICONST_5: &str = "iconst_5";
pub const BIPUSH: &str = "bipush";
pub const SIPUSH: &str = "sipush";
pub const LDC: &str = "ldc";

pub const NEW: &str = "new";
pub const POP: &str = "pop";
pub const DUP: &str = "dup";
pub const DUPX1: &str = "dup_x1";
pub const DUP2: &str = "dup2";
pub const SWAP: &str = "swap";
pub const I_ADD: &str = "iadd";
pub const I_SUB: &str = "isub";
pub const I_MUL: &str = "imul";
pub const I_DIV: &str = "idiv";

pub const IF_NE: &str = "ifne";
pub const IF_EQ: &str = "ifeq";
pub const IF_LT: &str = "iflt";
pub const IF_LE: &str = "ifle";
pub const IF_GT: &str = "ifgt";
pub const IF_GE: &str = "ifge";
pub const GOTO: &str = "goto";

pub const LOOKUP_SWITCH: &str = "lookupswitch";
pub const DEFAULT: &str = "default";

pub const INVOKE_STATIC: &str = "invokestatic";
pub const INVOKE_VIRTUAL: &str = "invokevirtual";
pub const INVOKE_INTERFACE: &str = "invokeinterface";
pub const INVOKE_SPECIAL: &str = "invokespecial";

pub const I_STORE: &str = "istore";
pub const A_STORE: &str = "astore";
pub const I_LOAD: &str = "iload";
pub const A_LOAD: &str = "aload";

pub const GET_STATIC: &str = "getstatic";

pub const DIR_STACK_LIMIT: &str = ".limit stack";
pub const DIR_LOCALS_LIMIT: &str = ".limit locals";
pub const DIR_SOURCE_FILE: &str = ".source";
pub const DIR_LINE: &str = ".line";

pub const TYPE_PRINT_STREAM: &str = "Ljava/io/PrintStream;";
pub const TYPE_CONSOLE: &str = "Ljava/io/Console;";
pub const TYPE_INT: &str = "I";
pub const TYPE_OBJECT: &str = "Ljava/lang/Object;";
pub const TYPE_STRING: &str = "Ljava/lang/String;";
pub const TYPE_INTEGER: &str = "Ljava/lang/Integer;";
pub const TYPE_VOID: &str = "V";
pub const TYPE_BOOL: &str = "Z";

pub const CHECK_CAST: &str = "checkcast";

pub const CLASS_OBJECT: &str = "java/lang/Object";
pub const CLASS_STRING: &str = "java/lang/String";
pub const CLASS_INTEGER: &str = "java/lang/Integer";
pub const CLASS_ARRAY_LIST: &str = "java/util/ArrayList";

pub const OBJ_SYSTEM_OUT: &str = "java/lang/System/out";
pub const OBJ_SYSTEM_CONSOLE: &str = "java/lang/System/console";
