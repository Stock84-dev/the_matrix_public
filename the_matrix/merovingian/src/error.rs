use thiserror::Error;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("Test failed.")]
    TestFailed,
}
