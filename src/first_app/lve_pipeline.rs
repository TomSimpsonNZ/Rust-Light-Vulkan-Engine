pub struct LvePipeline {}

impl LvePipeline {
    pub fn new(
        vert_file_path: &str, 
        frag_file_path: &str,
    ) -> Self {
        Self::create_graphics_pipeline(vert_file_path, frag_file_path);

        Self {}
    }

    fn read_file<P: AsRef<std::path::Path>>(file_path: P) -> Vec<u32> {
        log::debug!("Loading shader file {}", file_path.as_ref().to_str().unwrap());
        let mut file = std::fs::File::open(file_path).map_err(|e| log::error!("Unable to open file: {}", e)).unwrap();
        ash::util::read_spv(&mut file).map_err(|e| log::error!("Unable to read file: {}", e)).unwrap()
    }

    fn create_graphics_pipeline(
        vert_file_path: &str, 
        frag_file_path: &str,
    ) {

        let vert_code = Self::read_file(vert_file_path);
        let frag_code = Self::read_file(frag_file_path);

        log::info!("Vertex shade code size: {}", vert_code.len());
        log::info!("Fragment shade code size: {}", frag_code.len());
    }
}

impl Drop for LvePipeline {
    fn drop(&mut self) {
        log::debug!("Dropping pipeline");        
    }
}