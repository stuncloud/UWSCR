
use std::vec;

use quote::{quote, ToTokens};
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream, ParseBuffer},
    punctuated::Punctuated,
    token::{Brace, Bracket},
    Error, Ident, LitStr, LitInt,
    Token
};
use proc_macro2::TokenStream as TokenStream2;

/// 組み込み関数の詳細
#[derive(Debug)]
pub struct FuncDesc {
    /// 関数の説明
    pub desc: String,
    /// 引数の説明
    pub args: Option<Args>,
    /// 戻り値の説明
    pub rtype: Option<RetDesc>,
}

pub struct ParamsAndDocument {
    pub params: String,
    pub document: String,
    pub label_desc: Option<String>,
}
impl ParamsAndDocument {
    fn new(params: String, document: String, label_desc: Option<String>) -> Self {
        Self { params, document, label_desc }
    }
}

#[derive(Debug)]
pub enum Args {
    Args(Vec<ArgDesc>),
    Sets(Vec<(Vec<ArgDesc>, Option<String>)>),
}

impl Args {
    pub fn as_params_and_document(&self) -> Vec<ParamsAndDocument> {
        match self {
            Args::Args(args) => {
                let params = Self::to_params(args);
                let document = Self::to_document(args);
                vec![ParamsAndDocument::new(params, document, None)]
            },
            Args::Sets(v) => {
                v.iter().map(|(args, detail)| {
                    let params = Self::to_params(args);
                    let document = Self::to_document(args);
                    let label_desc = detail.clone();
                    let document = match detail {
                        Some(detail) => format!("{detail}\n{document}"),
                        None => document,
                    };
                    ParamsAndDocument::new(params, document, label_desc)
                })
                .collect()
            },
        }
    }
    fn to_params(vec: &Vec<ArgDesc>) -> String {
        vec.iter().map(|arg| arg.name.clone()).reduce(|a, b| a + ", " + &b).unwrap_or_default()
    }
    fn to_document(vec: &Vec<ArgDesc>) -> String {
        vec.iter().map(|arg| {
            format!(
                "#### {} [{}/{}{}]\n\n{}",
                arg.name,
                if arg.optional {"省略可"} else {"必須"},
                arg.r#type,
                arg.variadic.map(|n| format!("/可変長(最大: {n})")).unwrap_or_default(),
                arg.desc
            )
        })
        .reduce(|a,b| a + "\n\n" + &b)
        .unwrap_or_default()
    }
}

/// 組み込み関数の引数の詳細
#[derive(Debug)]
pub struct ArgDesc {
    /// 引数名
    pub name: String,
    /// 引数の取りうる型
    pub r#type: String,
    /// 引数の説明
    pub desc: String,
    /// オプション引数かどうか
    pub optional: bool,
    /// 可変長引数
    pub variadic: Option<u8>,
}
/// 組み込み関数の戻り値の詳細
#[derive(Debug)]
pub struct RetDesc {
    /// 戻り値の取りうる型
    pub r#type: String,
    /// 戻り値の説明
    pub desc: String,
}

impl FuncDesc {
    pub fn arg_len(&self) -> i32 {
        match &self.args {
            Some(args) => args.len(),
            None => 0,
        }
    }
}
impl Args {
    fn _len(args: &Vec<ArgDesc>) -> i32 {
        args.iter()
            .map(|arg| arg.variadic.unwrap_or(1))
            .reduce(|a,b| a + b)
            .unwrap_or_default() as i32
    }
    fn len(&self) -> i32 {
        match self {
            Args::Args(args) => Self::_len(args),
            Args::Sets(sets) => {
                sets.iter()
                    .map(|(args, _)| Self::_len(args))
                    .reduce(|a, b| a.max(b))
                    .unwrap_or_default() as i32
            },
        }
    }
}

fn parse_argdesc<'a>(content: ParseBuffer<'a>) -> syn::Result<Vec<ArgDesc>> {
    let mut args = vec![];
    loop {
        if content.is_empty() {
            break;
        }
        let content2;
        let _: Brace = braced!(content2 in content);
        let mut arg = ArgDesc { name: String::new(), r#type: String::new(), desc: String::new(), optional: false, variadic: None };
        loop {
            let ident: Ident = content2.parse()?;
            match ident.to_string().as_str() {
                "n" |
                "name" => {
                    let _equal: Token![=] = content2.parse()?;
                    let name: LitStr = content2.parse()?;
                    arg.name = name.value();
                },
                "t" |
                "types" => {
                    let _equal: Token![=] = content2.parse()?;
                    let r#type: LitStr = content2.parse()?;
                    arg.r#type = r#type.value();
                },
                "d" |
                "desc" => {
                    let _equal: Token![=] = content2.parse()?;
                    let desc: LitStr = content2.parse()?;
                    arg.desc = desc.value();
                },
                "o" |
                "optional" => {
                    arg.optional = true;
                },
                "v" |
                "variadic" => {
                    let _equal: Token![=] = content2.parse()?;
                    let n: LitInt = content2.parse()?;
                    arg.variadic = Some(n.base10_parse()?);
                }
                i => {
                    return Err(Error::new(ident.span(), format!("invalid identifier: {i}")));
                }
            }
            if content2.peek(Token![,]) {
                let _comma: Token![,] = content2.parse()?;
            }
            if content2.is_empty() {
                break;
            }
        }
        args.push(arg);
        if content.peek(Token![,]) {
            let _comma: Token![,] = content.parse()?;
        }
    }
    Ok(args)
}

impl Parse for FuncDesc {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut desc = String::new();
        let mut args = None;
        let mut rtype = None;
        loop {
            // 識別子
            let ident: Ident = input.parse()?;
            // =
            let _equal: Token![=] = input.parse()?;
            match ident.to_string().as_str() {
                "desc" => {
                    let str: LitStr = input.parse()?;
                    desc = str.value();
                },
                "args" => {
                    if args.is_some() {
                        return Err(Error::new(ident.span(), "duplicated"));
                    }
                    let content;
                    let _: Bracket = bracketed!(content in input);
                    let _args = parse_argdesc(content)?;
                    args = Some(Args::Args(_args));
                },
                /*
                    sets=[
                        "説明",
                        [
                            {n="a"},
                            {n="b",o}
                        ],
                        "説明",
                        [
                            {n="a"},
                            {n="c"},
                            {n="d",o}
                        ]
                    ]
                 */
                "sets" => {
                    if args.is_some() {
                        return Err(Error::new(ident.span(), "duplicated"));
                    }
                    let content;
                    let _: Bracket = bracketed!(content in input);
                    let mut sets = vec![];
                    loop {
                        if content.is_empty() {
                            break;
                        }
                        let detail = if content.peek(LitStr) {
                            let s: LitStr = content.parse()?;
                            if content.peek(Token![,]) {
                                let _comma: Token![,] = content.parse()?;
                            }
                            Some(s.value())
                        } else {
                            None
                        };
                        let content2;
                        let _: Bracket = bracketed!(content2 in content);
                        let _args = parse_argdesc(content2)?;
                        sets.push((_args, detail));
                        if content.peek(Token![,]) {
                            let _comma: Token![,] = content.parse()?;
                        }
                    }
                    args = Some(Args::Sets(sets));
                }
                "rtype" => {
                    let content;
                    let _: Brace = braced!(content in input);
                    let mut desc = String::new();
                    let mut r#type = String::new();
                    loop {
                        let ident: Ident = content.parse()?;
                        let _equal: Token![=] = content.parse()?;
                        match ident.to_string().as_str() {
                            "desc" => {
                                let s: LitStr = content.parse()?;
                                desc = s.value();
                            },
                            "types" => {
                                let s: LitStr = content.parse()?;
                                r#type = s.value();
                            },
                            i => {
                                return Err(Error::new(ident.span(), format!("invalid identifier: {i}")));
                            }
                        }
                        if content.peek(Token![,]) {
                            let _comma: Token![,] = content.parse()?;
                        }
                        if content.is_empty() {
                            break;
                        }
                    }
                    rtype = Some(RetDesc { r#type, desc })
                },
                i => {
                    return Err(Error::new(ident.span(), format!("invalid identifier: {i}")));
                }
            }
            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
            }
            if input.is_empty() {
                break;
            }
        }
        Ok(Self { desc, args, rtype })
    }
}

impl ToTokens for FuncDesc {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { desc, args, rtype } = self;
        let _args = match args {
            Some(args) => match args {
                Args::Args(args) => {
                    let mut punct: Punctuated<TokenStream2, Token![,]> = Punctuated::new();
                    args.iter()
                        .map(|arg| quote! {#arg})
                        .for_each(|arg| punct.push(arg) );
                    quote! {
                        Some(Args::Args(vec![
                            #punct
                        ]))
                    }
                },
                Args::Sets(sets) => {
                    let mut punct: Punctuated<TokenStream2, Token![,]> = Punctuated::new();
                    for (set, detail) in sets {
                        let mut punct2: Punctuated<TokenStream2, Token![,]> = Punctuated::new();
                        set.iter()
                            .map(|arg| quote! {#arg})
                            .for_each(|arg| punct2.push(arg) );
                        let detail = match detail {
                            Some(d) => quote! {
                                Some(#d.to_string())
                            },
                            None => quote! {
                                None
                            },
                        };
                        let token = quote! {
                            (vec![#punct2], #detail)
                        };
                        punct.push(token);
                    }
                    quote! {
                        Some(Args::Sets(vec![
                            #punct
                        ]))
                    }
                },
            },
            None => quote! {
                None
            },
        };
        let _type = match rtype {
            Some(r) => quote! {
                Some(#r)
            },
            None => quote! {
                None
            },
        };
        let stream: TokenStream2 = quote! {
            FuncDesc {
                desc: #desc.to_string(),
                args: #_args,
                rtype: #_type,
            }
        }.into();
        tokens.extend(stream.into_iter());
    }
}
impl ToTokens for ArgDesc {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { name, r#type, desc, optional, variadic } = self;
        let opt_variadic = match variadic {
            Some(n) => quote! {
                Some(#n)
            },
            None => quote! {
                None
            },
        };
        let stream: TokenStream2 = quote! {
            ArgDesc {
                name: #name.to_string(),
                r#type: #r#type.to_string(),
                desc: #desc.to_string(),
                optional: #optional,
                variadic: #opt_variadic
            }
        }.into();
        tokens.extend(stream.into_iter())
    }
}
impl ToTokens for RetDesc {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { r#type, desc } = self;
        let stream: TokenStream2 = quote! {
            RetDesc {
                r#type: #r#type.to_string(),
                desc: #desc.to_string(),
            }
        }.into();
        tokens.extend(stream.into_iter())
    }
}