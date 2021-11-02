use ash::version::DeviceV1_0;
use ash::{vk, Device};

pub struct PipelineConfigInfo {}

pub struct LvePipeline {
    graphics_pipeline: vk::Pipeline,
    vert_shader_module: vk::ShaderModule,
    frag_shader_module: vk::ShaderModule,
}

impl LvePipeline {
    pub fn new(
        device: &Device,
        vert_file_path: &str, 
        frag_file_path: &str,
        config_info: &PipelineConfigInfo,
    ) -> Self {
        let (graphics_pipeline, vert_shader_module, frag_shader_module) =
            Self::create_graphics_pipeline(device, vert_file_path, frag_file_path, config_info);

        Self {
            graphics_pipeline,
            vert_shader_module,
            frag_shader_module,
        }
    }

    pub unsafe fn destructor(&mut self, device: &Device) {
        
        device.destroy_shader_module(self.vert_shader_module, None);
        device.destroy_shader_module(self.frag_shader_module, None);
        device.destroy_pipeline(self.graphics_pipeline, None);
        
    }

    pub fn default_pipline_config_info(_width: u32, _height: u32) -> PipelineConfigInfo {
        PipelineConfigInfo {}
    }

    fn read_file<P: AsRef<std::path::Path>>(file_path: P) -> Vec<u32> {
        log::debug!("Loading shader file {}", file_path.as_ref().to_str().unwrap());
        let mut file = std::fs::File::open(file_path).map_err(|e| log::error!("Unable to open file: {}", e)).unwrap();
        ash::util::read_spv(&mut file).map_err(|e| log::error!("Unable to read file: {}", e)).unwrap()
    }

    fn create_graphics_pipeline(
        _device: &Device,
        vert_file_path: &str, 
        frag_file_path: &str,
        _config_info: &PipelineConfigInfo,
    ) -> (vk::Pipeline, vk::ShaderModule, vk::ShaderModule) {

        let vert_code = Self::read_file(vert_file_path);
        let frag_code = Self::read_file(frag_file_path);

        log::info!("Vertex shader code size: {}", vert_code.len());
        log::info!("Fragment shader code size: {}", frag_code.len());

        (vk::Pipeline::null(), vk::ShaderModule::null(), vk::ShaderModule::null())
    }

    fn _create_shader_module(device: &Device, code: &Vec<u32>) -> vk::ShaderModule {
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(code)
            .build();
        
        unsafe {
            device.create_shader_module(&create_info, None)
                .map_err(|e| log::error!("Unable to create shader module: {}", e))
                .unwrap()
        }
    }
}

impl Drop for LvePipeline {
    fn drop(&mut self) {
        log::debug!("Dropping pipeline");        
    }
}