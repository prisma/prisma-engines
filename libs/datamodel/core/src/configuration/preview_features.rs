pub trait PreviewFeatures: Send + Sync {
    fn preview_features(&self) -> &Vec<String>;

    fn has_preview_feature(&self, feature: &str) -> bool {
        self.preview_features().contains(&feature.to_string())
    }
}
