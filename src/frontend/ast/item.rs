use crate::frontend::ast::printer::{AstPrint, branch, next_prefix};
use crate::frontend::ast::stmt::Block;
use crate::frontend::ast::typ::Type;
use std::fmt::Write;

#[derive(Debug)]
pub enum Item {
    FunctionDef(FunctionDef),
    FunctionDecl(FunctionDecl),
    Class(Class),
    Trait(Trait),
    Impl(Impl),
}

#[derive(Debug)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
}

#[derive(Debug)]
pub struct FunctionDef {
    pub function_header: FunctionDecl,
    pub body: Block,
}


#[derive(Debug)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug)]
pub struct Class {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug)]
pub struct Trait {
    pub name: String,
    pub methods: Vec<FunctionDecl>,
}

#[derive(Debug)]
pub struct Impl {
    pub trait_name: String,
    pub class_name: String,
    pub methods: Vec<FunctionDef>,
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

// ---- Item 系列 ----

impl AstPrint for Item {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        match self {
            Item::FunctionDef(f) => f.print(prefix, is_last, output),
            Item::FunctionDecl(d) => d.print(prefix, is_last, output),
            Item::Class(c) => c.print(prefix, is_last, output),
            Item::Trait(t) => t.print(prefix, is_last, output),
            Item::Impl(i) => i.print(prefix, is_last, output),
        }
    }
}

impl AstPrint for FunctionDecl {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        let branch_str = branch(is_last);
        write!(output, "{}{}FunctionDecl({}", prefix, branch_str, self.name)?;
        write!(output, " params=[")?;
        for (i, p) in self.params.iter().enumerate() {
            if i > 0 { write!(output, ", ")?; }
            write!(output, "{}: {:?}", p.name, p.ty)?;
        }
        write!(output, "]")?;
        if let Some(ref ret) = self.return_type {
            write!(output, " -> {:?}", ret)?;
        }
        writeln!(output, ")")
    }
}

impl AstPrint for FunctionDef {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        let branch_str = branch(is_last);
        writeln!(output, "{}{}FunctionDef({})", prefix, branch_str, self.function_header.name)?;
        let child = next_prefix(prefix, is_last);
        // 先简单打印函数签名
        writeln!(output, "{}├── Signature:", child)?;
        self.function_header.print(&format!("{}│   ", child), true, output)?;
        writeln!(output, "{}└── Body:", child)?;
        self.body.print(&format!("{}    ", child), true, output)?;
        Ok(())
    }
}

impl AstPrint for Parameter {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        let branch_str = branch(is_last);
        writeln!(output, "{}{}Param({}: {:?})", prefix, branch_str, self.name, self.ty)
    }
}

impl AstPrint for Class {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        let branch_str = branch(is_last);
        writeln!(output, "{}{}Class({})", prefix, branch_str, self.name)?;
        let child = next_prefix(prefix, is_last);

        // fields
        if !self.fields.is_empty() {
            writeln!(output, "{}└── Fields:", child)?;
            let f_child = format!("{}    ", child);
            let count = self.fields.len();
            for (i, field) in self.fields.iter().enumerate() {
                field.print(&f_child, i == count - 1, output)?;
            }
        }
        Ok(())
    }
}

impl AstPrint for Field {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        let branch_str = branch(is_last);
        writeln!(output, "{}{}Field({}: {:?})", prefix, branch_str, self.name, self.ty)
    }
}

impl AstPrint for Trait {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        let branch_str = branch(is_last);
        writeln!(output, "{}{}Trait({})", prefix, branch_str, self.name)?;
        let child = next_prefix(prefix, is_last);

        // methods
        if !self.methods.is_empty() {
            writeln!(output, "{}└── Methods:", child)?;
            let m_child = format!("{}    ", child);
            let count = self.methods.len();
            for (i, method) in self.methods.iter().enumerate() {
                method.print(&m_child, i == count - 1, output)?;
            }
        }
        Ok(())
    }
    
}

impl AstPrint for Impl {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        let branch_str = branch(is_last);
        let trait_part = if self.trait_name.is_empty() {
            String::new()
        } else {
            format!("({})", self.trait_name)
        };
        writeln!(
            output,
            "{}{}Impl{} for {}",
            prefix, branch_str, trait_part, self.class_name
        )?;

        let child = next_prefix(prefix, is_last);

        if !self.methods.is_empty() {
            writeln!(output, "{}└── Methods:", child)?;
            let methods_prefix = format!("{}    ", child); // Methods: 下唯一分支的子项前缀
            let count = self.methods.len();
            for (i, method) in self.methods.iter().enumerate() {
                method.print(&methods_prefix, i == count - 1, output)?;
            }
        } 
        Ok(())
    }
}
