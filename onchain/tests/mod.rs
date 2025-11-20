use litesvm::LiteSVM;

struct TestSetup {
    pub lite_svm: LiteSVM,
}

fn setup() -> TestSetup {
    TestSetup {
        lite_svm: LiteSVM::new().with_default_programs(),
    }
}
