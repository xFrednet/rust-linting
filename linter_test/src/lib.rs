use linter_api::{lint::Lint, LintPass};

linter_api::interface::export_lint_pass!("linter_test", TestLintPass::new());

linter_api::lint::declare_lint!(TEST_LINT, Allow, "");

struct TestLintPass {}

impl TestLintPass {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'ast> LintPass<'ast> for TestLintPass {
    fn test_call(&self, msg: &str) {
        println!("Message from test: {}", msg);
    }

    fn registered_lints(&self) -> Vec<&'static Lint> {
        vec![TEST_LINT]
    }

    fn check_item(&mut self, item: &'ast dyn linter_api::ast::item::Item<'ast>) {
        match item.get_kind() {
            linter_api::ast::item::ItemKind::StaticItem(item) => {
                dbg!(item.get_type());
            },
            _ => {},
        }
    }
}
