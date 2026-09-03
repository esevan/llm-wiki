use fastembed::{
    InitOptionsUserDefined, Pooling, QuantizationMode, TextEmbedding, TokenizerFiles,
    UserDefinedEmbeddingModel,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const REQUIRED_FILES: [&str; 5] = [
    "model.onnx",
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
];

#[derive(Clone)]
pub struct SemanticEngine {
    model_dir: Option<PathBuf>,
    model: Arc<Mutex<Option<TextEmbedding>>>,
}

impl SemanticEngine {
    pub fn new(model_dir: Option<PathBuf>) -> Self {
        let model_dir = model_dir.filter(|path| Self::files_available(path));
        Self {
            model_dir,
            model: Arc::new(Mutex::new(None)),
        }
    }

    pub fn available(&self) -> bool {
        self.model_dir.is_some()
    }

    pub fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let mut model = self
            .model
            .lock()
            .map_err(|_| "Embedding model lock failed")?;
        if model.is_none() {
            *model = Some(self.load()?);
        }
        model
            .as_mut()
            .expect("embedding model initialized")
            .embed(texts, None)
            .map_err(|error| format!("Embedding inference failed: {error}"))
    }

    fn files_available(path: &Path) -> bool {
        REQUIRED_FILES.iter().all(|name| path.join(name).is_file())
    }

    fn load(&self) -> Result<TextEmbedding, String> {
        let root = self
            .model_dir
            .as_ref()
            .ok_or("The bundled embedding model is unavailable")?;
        let read = |name: &str| {
            fs::read(root.join(name))
                .map_err(|error| format!("Could not read bundled {name}: {error}"))
        };
        let tokenizer = TokenizerFiles {
            tokenizer_file: read("tokenizer.json")?,
            config_file: read("config.json")?,
            special_tokens_map_file: read("special_tokens_map.json")?,
            tokenizer_config_file: read("tokenizer_config.json")?,
        };
        let model = UserDefinedEmbeddingModel::new(read("model.onnx")?, tokenizer)
            .with_pooling(Pooling::Mean)
            .with_quantization(QuantizationMode::Static);
        TextEmbedding::try_new_from_user_defined(
            model,
            InitOptionsUserDefined::new().with_intra_threads(2),
        )
        .map_err(|error| format!("Could not load bundled embedding model: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_model_does_not_attempt_a_network_download() {
        let engine = SemanticEngine::new(Some(PathBuf::from("not-present")));
        assert!(!engine.available());
        assert_eq!(
            engine.embed(vec!["offline".into()]).unwrap_err(),
            "The bundled embedding model is unavailable"
        );
    }
}
