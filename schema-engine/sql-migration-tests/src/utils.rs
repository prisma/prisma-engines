use schema_core::json_rpc::types::SchemaContainer;

#[macro_export]
macro_rules! write_multi_file {
  // Match multiple pairs of filename and content
  ( $( $filename:expr => $content:expr ),* $(,)? ) => {
      {
          use std::fs::File;
          use std::io::Write;

          // Create a result vector to collect errors
          let mut results = Vec::new();
          let tmpdir = tempfile::tempdir().unwrap();

          std::fs::create_dir_all(&tmpdir).unwrap();

          $(
              let file_path = tmpdir.path().join($filename);
              // Attempt to create or open the file
              let result = (|| -> std::io::Result<()> {
                  let mut file = File::create(&file_path)?;
                  file.write_all($content.as_bytes())?;
                  Ok(())
              })();

              result.unwrap();

              results.push((file_path.to_string_lossy().into_owned(), $content));
          )*

          (tmpdir, results)
      }
  };
}

pub fn to_schema_containers(files: &[(String, &str)]) -> Vec<SchemaContainer> {
    files
        .iter()
        .map(|(path, content)| SchemaContainer {
            path: path.to_string(),
            content: content.to_string(),
        })
        .collect()
}
