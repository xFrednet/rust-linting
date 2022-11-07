use crate::ast::generic::GenericParams;
use crate::ast::{impl_callable_data_trait, BodyId, CommonCallableData};
use crate::ffi::FfiOption;

use super::CommonItemData;

/// A function item like:
///
/// ```
/// pub fn foo() {}
///
/// # pub struct SomeItem;
/// impl SomeItem {
///     pub fn bar(&self) {}
/// }
///
/// pub trait SomeTrait {
///     fn baz(_: i32);
/// }
/// ```
///
/// See: <https://doc.rust-lang.org/reference/items/functions.html>
#[repr(C)]
#[derive(Debug)]
pub struct FnItem<'ast> {
    data: CommonItemData<'ast>,
    generics: GenericParams<'ast>,
    callable_data: CommonCallableData<'ast>,
    body: FfiOption<BodyId>,
}

super::impl_item_data!(FnItem, Fn);

impl<'ast> FnItem<'ast> {
    pub fn generics(&self) -> &GenericParams<'ast> {
        &self.generics
    }

    pub fn body(&self) -> Option<BodyId> {
        self.body.get().copied()
    }
}

impl_callable_data_trait!(FnItem<'ast>);
