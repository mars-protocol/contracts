#[cfg(feature = "osmosis-test-tube")]
use cw_it::osmosis_test_tube::OsmosisTestApp;
use cw_it::{multi_test::MultiTestRunner, OwnedTestRunner};

const DEFAULT_TEST_RUNNER: &str = "multi-test";

/// Creates a test runner with the type defined by the TEST_RUNNER environment variable
pub fn get_test_runner<'a>() -> OwnedTestRunner<'a> {
    match option_env!("TEST_RUNNER").unwrap_or(DEFAULT_TEST_RUNNER) {
        #[cfg(feature = "osmosis-test-tube")]
        "osmosis-test-tube" => {
            let app = OsmosisTestApp::new();
            OwnedTestRunner::OsmosisTestApp(app)
        }
        "multi-test" => OwnedTestRunner::MultiTest(MultiTestRunner::new("osmo")),
        x => panic!("Unsupported test runner type {} specified", x),
    }
}
