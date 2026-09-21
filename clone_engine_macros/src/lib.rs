use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, FnArg, Ident, ItemFn, LitInt, Pat, ReturnType, Token, Type};

struct HookArgs {
    offset: Option<Expr>,
    slot: Option<Expr>,
    article: Option<Expr>,
    item: Option<Expr>,
    me: Option<Ident>,
    of: Option<Expr>,
    expect: Option<Expr>,
}

impl Parse for HookArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = HookArgs {
            offset: None,
            slot: None,
            article: None,
            item: None,
            me: None,
            of: None,
            expect: None,
        };
        let pairs = Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated(input)?;
        for pair in pairs {
            let key = pair
                .path
                .get_ident()
                .map(|ident| ident.to_string())
                .unwrap_or_default();
            match key.as_str() {
                "offset" => args.offset = Some(pair.value),
                "slot" => args.slot = Some(pair.value),
                "article" => args.article = Some(pair.value),
                "item" => args.item = Some(pair.value),
                "me" => {
                    let Expr::Path(path) = &pair.value else {
                        return Err(syn::Error::new_spanned(pair.value, "me = <parameter name>"));
                    };
                    args.me = path.path.get_ident().cloned();
                }
                "of" => args.of = Some(pair.value),
                "expect" => args.expect = Some(pair.value),
                "replace" => {
                    return Err(syn::Error::new_spanned(
                        pair.path,
                        "replace is not supported: the shared broker hooks main's text by offset; use #[skyline::hook(replace = ...)] with FIGHTER.is(...) inside, which is exclusive to your pack",
                    ))
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        pair.path,
                        "unknown key; use offset or slot (with article for a weapon's), me, of, expect",
                    ))
                }
            }
        }
        Ok(args)
    }
}

fn is_float(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.path.is_ident("f32") || path.path.is_ident("f64"))
}

fn is_by_value_struct(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => {
            let Some(last) = path.path.segments.last() else {
                return true;
            };
            let name = last.ident.to_string();
            !matches!(
                name.as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "isize"
                    | "bool"
                    | "f32"
                    | "f64"
            )
        }
        Type::Ptr(_) => false,
        Type::Reference(_) => true,
        Type::Tuple(tuple) => !tuple.elems.is_empty(),
        _ => true,
    }
}

struct Signature {
    names: Vec<Ident>,
    types: Vec<Type>,
    output: Type,
}

fn signature(function: &ItemFn, allow_float: bool) -> Result<Signature, syn::Error> {
    let mut names = Vec::new();
    let mut types = Vec::new();
    for input in &function.sig.inputs {
        let FnArg::Typed(typed) = input else {
            return Err(syn::Error::new_spanned(input, "a hook takes plain parameters, not self"));
        };
        let Pat::Ident(ident) = &*typed.pat else {
            return Err(syn::Error::new_spanned(&typed.pat, "each parameter needs a plain name"));
        };
        if !allow_float && is_float(&typed.ty) {
            return Err(syn::Error::new_spanned(
                &typed.ty,
                "the shared broker passes x0..x5 only: a float parameter cannot be hooked by offset; a vtable slot (slot = ...) takes it",
            ));
        }
        if is_by_value_struct(&typed.ty) {
            return Err(syn::Error::new_spanned(
                &typed.ty,
                "hook parameters must be integers, floats or raw pointers (a reference or by-value struct is not a register); use #[skyline::hook] with FIGHTER.is(...) inside, which is exclusive to your pack",
            ));
        }
        names.push(ident.ident.clone());
        types.push((*typed.ty).clone());
    }
    let output: Type = match &function.sig.output {
        ReturnType::Default => syn::parse_quote!(()),
        ReturnType::Type(_, ty) => (**ty).clone(),
    };
    if !allow_float && is_float(&output) {
        return Err(syn::Error::new_spanned(
            &output,
            "the shared broker returns x0 only: a float return cannot be hooked by offset; a vtable slot (slot = ...) returns it",
        ));
    }
    if is_by_value_struct(&output) {
        return Err(syn::Error::new_spanned(
            &output,
            "the hook must return an integer, a float, a raw pointer or nothing",
        ));
    }
    Ok(Signature { names, types, output })
}

fn slot_override(args: &HookArgs, function: &ItemFn, site: proc_macro2::TokenStream) -> TokenStream {
    if let Some(me) = &args.me {
        return syn::Error::new_spanned(
            me,
            "me is not needed with slot = ...: the entry lives in this fighter's own vtable and only its objects reach it",
        )
        .to_compile_error()
        .into();
    }
    if let Some(expect) = &args.expect {
        return syn::Error::new_spanned(
            expect,
            "expect = [...] pins the words of a hooked address; a vtable slot has none",
        )
        .to_compile_error()
        .into();
    }
    let sig = match signature(function, true) {
        Ok(sig) => sig,
        Err(error) => return error.to_compile_error().into(),
    };
    let Signature { names, types, output } = sig;
    let name = function.sig.ident.clone();
    let entry_name = format_ident!("__clone_engine_entry_{}", name);
    let vis = function.vis.clone();
    let body = function.block.clone();

    let expanded = quote! {
        #[allow(non_upper_case_globals)]
        #vis static #name: ::clone_engine_api::v2::Override = ::clone_engine_api::v2::Override::new(
            #site,
            #entry_name as *const (),
            stringify!(#name),
        );

        #[allow(unused_variables, unused_mut, non_snake_case, clippy::all)]
        unsafe extern "C" fn #entry_name(#(#names: #types),*) -> #output {
            #[allow(unused_macros)]
            macro_rules! call_original {
                ($($arg:expr),* $(,)?) => {{
                    let __original = #name.original();
                    if __original == 0 {
                        ::clone_engine_api::elog!(
                            "[clone_engine] {}: call_original! before the slot was taken",
                            stringify!(#name)
                        );
                        return ::core::mem::zeroed();
                    }
                    let __f: unsafe extern "C" fn(#(#types),*) -> #output = ::core::mem::transmute(__original);
                    __f($($arg),*)
                }};
            }
            #body
        }
    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn hook(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as HookArgs);
    let function = syn::parse_macro_input!(item as ItemFn);

    let fighter_of = match &args.of {
        Some(of) => quote!(#of),
        None => quote!(crate::__CLONE_ENGINE_FIGHTER),
    };
    if let Some(item) = &args.item {
        if args.article.is_some() || args.offset.is_some() {
            return syn::Error::new_spanned(
                item,
                "item = ITEM goes with slot = slot::item::... alone",
            )
            .to_compile_error()
            .into();
        }
        let Some(slot) = &args.slot else {
            return syn::Error::new_spanned(item, "item = ITEM needs slot = slot::item::...")
                .to_compile_error()
                .into();
        };
        let site = quote!(::clone_engine_api::v2::Site::ItemSlot(&#item, (#slot) as u32));
        return slot_override(&args, &function, site);
    }
    let site = match (&args.offset, &args.slot, &args.article) {
        (Some(offset), None, None) => quote!(::clone_engine_api::v2::Site::Offset((#offset) as u64)),
        (None, Some(slot), None) => {
            let site = quote!(::clone_engine_api::v2::Site::FighterSlot(#fighter_of, (#slot) as u32));
            return slot_override(&args, &function, site);
        }
        (None, Some(slot), Some(article)) => {
            let site =
                quote!(::clone_engine_api::v2::Site::WeaponSlot(#fighter_of, #article, (#slot) as u32));
            return slot_override(&args, &function, site);
        }
        (Some(_), Some(_), _) => {
            return syn::Error::new_spanned(
                &function.sig.ident,
                "give offset = 0x... or slot = ..., not both",
            )
            .to_compile_error()
            .into();
        }
        (_, None, Some(_)) => {
            return syn::Error::new_spanned(
                &function.sig.ident,
                "article = \"name\" needs slot = slot::weapon::...",
            )
            .to_compile_error()
            .into();
        }
        (None, None, None) => {
            return syn::Error::new_spanned(
                &function.sig.ident,
                "offset = 0x... or slot = slot::fighter::... is required",
            )
            .to_compile_error()
            .into();
        }
    };

    let name = function.sig.ident.clone();
    let body_name = format_ident!("__clone_engine_body_{}", name);
    let shim_name = format_ident!("__clone_engine_shim_{}", name);
    let vis = function.vis.clone();
    let body = function.block.clone();

    let sig = match signature(&function, false) {
        Ok(sig) => sig,
        Err(error) => return error.to_compile_error().into(),
    };
    let Signature {
        names: param_names,
        types: param_types,
        output: return_type,
    } = sig;
    if param_names.len() > 6 {
        return syn::Error::new_spanned(
            &function.sig.inputs,
            "the shared broker passes x0..x5: at most six parameters; use #[skyline::hook] with FIGHTER.is(...) inside, which is exclusive to your pack",
        )
        .to_compile_error()
        .into();
    }
    let argument_count = param_names.len() as u32;

    let gate = match &args.me {
        Some(me) => {
            if !param_names.iter().any(|param| param == me) {
                return syn::Error::new_spanned(me, "me must name one of the parameters")
                    .to_compile_error()
                    .into();
            }
            quote! {
                if !(#fighter_of).is_or_owns(#me) {
                    return ::clone_engine_api::HOOK_DECLINED;
                }
            }
        }
        None => quote!(),
    };

    let expect = match args.expect {
        Some(expect) => quote!(::core::option::Option::Some(#expect)),
        None => quote!(::core::option::Option::None),
    };

    let unpack = param_names.iter().zip(param_types.iter()).enumerate().map(|(index, (param, ty))| {
        quote! { let #param: #ty = ::clone_engine_api::v2::FromRaw::from_raw(call.args[#index]); }
    });
    let count = LitInt::new(&argument_count.to_string(), proc_macro2::Span::call_site());

    let expanded = quote! {
        #[allow(non_upper_case_globals)]
        #vis static #name: ::clone_engine_api::v2::Hook = ::clone_engine_api::v2::Hook::at(
            #site,
            #count,
            #expect,
            #shim_name,
            stringify!(#name),
        );

        #[allow(unused_variables, unused_mut, non_snake_case, clippy::all)]
        unsafe fn #body_name(#(#param_names: #param_types),*) -> #return_type {
            #[allow(unused_macros)]
            macro_rules! call_original {
                ($($arg:expr),* $(,)?) => {{
                    let mut __args: [u64; 6] = [0; 6];
                    let mut __index = 0usize;
                    $(
                        __args[__index] = ::clone_engine_api::v2::IntoRaw::into_raw($arg);
                        __index += 1;
                    )*
                    let __raw = ::clone_engine_api::shared_hook_original(#name.offset(), &__args);
                    <#return_type as ::clone_engine_api::v2::FromRaw>::from_raw(__raw)
                }};
            }
            #body
        }

        #[allow(non_snake_case)]
        unsafe extern "C" fn #shim_name(call: *mut ::clone_engine_api::HookCall) -> u32 {
            if call.is_null() {
                return ::clone_engine_api::HOOK_DECLINED;
            }
            let call = &mut *call;
            #(#unpack)*
            #gate
            let __result: #return_type = #body_name(#(#param_names),*);
            call.result = ::clone_engine_api::v2::IntoRaw::into_raw(__result);
            ::clone_engine_api::HOOK_HANDLED
        }
    };
    expanded.into()
}
