use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input,
    Ident, ItemFn,
};
use proc_macro2::Span;
use func_desc::FuncDesc;

#[proc_macro_attribute]
pub fn builtin_func_desc(args: TokenStream, input: TokenStream) -> TokenStream {
    // 属性の引数を得る
    let desc = parse_macro_input!(args as FuncDesc);

    // *_desc()関数定義を作る
    let input2 = input.clone();
    let item_fn = parse_macro_input!(input2 as ItemFn);
    let fn_name = ident_to_func_desc_name(item_fn.sig.ident);
    let mut fn_desc: TokenStream = quote! {
        fn #fn_name() -> FuncDesc {
            #desc
        }
    }.into();

    // 元の関数定義と*_desc関数定義を返す
    fn_desc.extend(input);
    fn_desc
}

#[proc_macro]
pub fn get_desc(item: TokenStream) -> TokenStream {
    let ident = parse_macro_input!(item as Ident);
    let fn_name = ident_to_func_desc_name(ident);
    quote! {
        #fn_name()
    }.into()
}

fn ident_to_func_desc_name(ident: Ident) -> Ident {
    let fn_name = format!("{}_desc", ident);
    Ident::new(&fn_name, Span::call_site())
}