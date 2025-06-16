pub trait AsyncDefault {
    async fn default() -> Self;
}

impl<T: Default> AsyncDefault for T {
    async fn default() -> Self {
        Default::default()
    }
}
