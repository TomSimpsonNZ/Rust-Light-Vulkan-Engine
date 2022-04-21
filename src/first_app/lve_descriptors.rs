use ash::vk;

use super::lve_device::LveDevice;

use std::collections::HashMap;
use std::rc::Rc;

pub struct LveDescriptorSetLayout {
    lve_device: Rc<LveDevice>,
    bindings: HashMap<u32, vk::DescriptorSetLayoutBinding>,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
}

impl LveDescriptorSetLayout {
    pub fn new(
        lve_device: Rc<LveDevice>,
        bindings: HashMap<u32, vk::DescriptorSetLayoutBinding>,
    ) -> Rc<LveDescriptorSetLayout> {
        let mut set_layout_bindings: Vec<vk::DescriptorSetLayoutBinding> = Vec::new();

        bindings.iter().for_each(|(_, binding)| {
            set_layout_bindings.push(*binding);
        });

        let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(set_layout_bindings.as_slice())
            .build();

        let descriptor_set_layout = unsafe {
            lve_device
                .device
                .create_descriptor_set_layout(&descriptor_set_layout_info, None)
                .map_err(|e| log::error!("failed to create descriptor set layout: {}", e))
                .unwrap()
        };

        Rc::new(LveDescriptorSetLayout {
            lve_device,
            bindings,
            descriptor_set_layout,
        })
    }
}

impl Drop for LveDescriptorSetLayout {
    fn drop(&mut self) {
        log::debug!("Dropping Descriptor Set Layout");
        unsafe {
            self.lve_device
                .device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

pub struct LveDescriptorSetLayoutBuilder {
    lve_device: Rc<LveDevice>,
    bindings: HashMap<u32, vk::DescriptorSetLayoutBinding>,
}

impl LveDescriptorSetLayoutBuilder {
    pub fn new(lve_device: Rc<LveDevice>) -> LveDescriptorSetLayoutBuilder {
        LveDescriptorSetLayoutBuilder {
            lve_device,
            bindings: HashMap::<u32, vk::DescriptorSetLayoutBinding>::new(),
        }
    }

    pub fn add_binding<'a>(
        &'a mut self,
        binding: u32,
        descriptor_type: vk::DescriptorType,
        stage_flags: vk::ShaderStageFlags,
        count: u32,
    ) -> &'a mut LveDescriptorSetLayoutBuilder {
        assert!(
            !self.bindings.contains_key(&binding),
            "Binding already in use"
        );
        let layout_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(binding)
            .descriptor_type(descriptor_type)
            .descriptor_count(count)
            .stage_flags(stage_flags)
            .build();

        self.bindings.insert(binding, layout_binding);

        self
    }

    pub fn build(&self) -> Rc<LveDescriptorSetLayout> {
        LveDescriptorSetLayout::new(Rc::clone(&self.lve_device), HashMap::clone(&self.bindings))
    }
}

pub struct LveDescriptorPool {
    lve_device: Rc<LveDevice>,
    descriptor_pool: vk::DescriptorPool,
}

impl LveDescriptorPool {
    pub fn new(
        lve_device: Rc<LveDevice>,
        max_sets: u32,
        pool_flags: vk::DescriptorPoolCreateFlags,
        pool_sizes: &Vec<vk::DescriptorPoolSize>,
    ) -> Rc<LveDescriptorPool> {
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(pool_sizes.as_slice())
            .max_sets(max_sets)
            .flags(pool_flags)
            .build();

        let descriptor_pool = unsafe {
            lve_device
                .device
                .create_descriptor_pool(&descriptor_pool_info, None)
                .map_err(|e| log::error!("Failed to create descriptor pool: {}", e))
                .unwrap()
        };

        Rc::new(LveDescriptorPool {
            lve_device,
            descriptor_pool,
        })
    }

    fn allocate_descriptor(
        &self,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<vk::DescriptorSet, ()> {
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&[descriptor_set_layout])
            .build();

        // Might want to create a "DescriptorPoolManager" class that handles this case, and builds
        // a new pool whenever an old pool fills up. But this is beyond our current scope
        let descriptor_set_result =
            unsafe { self.lve_device.device.allocate_descriptor_sets(&alloc_info) };

        match descriptor_set_result {
            Ok(descriptor_set) => return Ok(descriptor_set[0]),
            Err(_) => return Err(()),
        }
    }

    pub unsafe fn free_descriptors(&self, descriptors: &Vec<vk::DescriptorSet>) {
        self.lve_device
            .device
            .free_descriptor_sets(self.descriptor_pool, descriptors.as_slice())
            .map_err(|e| log::error!("Failed to free descriptor sets: {}", e))
            .unwrap()
    }

    pub unsafe fn reset_pool(&self) {
        self.lve_device
            .device
            .reset_descriptor_pool(self.descriptor_pool, vk::DescriptorPoolResetFlags::empty())
            .map_err(|e| log::error!("Failed to reset descriptor pool: {}", e))
            .unwrap()
    }
}

impl Drop for LveDescriptorPool {
    fn drop(&mut self) {
        log::debug!("Dropping Descriptor Pool");
        unsafe {
            self.lve_device
                .device
                .destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}

pub struct LveDescriptorPoolBuilder {
    lve_device: Rc<LveDevice>,
    pool_sizes: Vec<vk::DescriptorPoolSize>,
    max_sets: u32,
    pool_flags: vk::DescriptorPoolCreateFlags,
}

impl LveDescriptorPoolBuilder {
    pub fn new(lve_device: Rc<LveDevice>) -> LveDescriptorPoolBuilder {
        LveDescriptorPoolBuilder {
            lve_device,
            pool_sizes: Vec::<vk::DescriptorPoolSize>::new(),
            max_sets: 1000,
            pool_flags: vk::DescriptorPoolCreateFlags::empty(),
        }
    }

    pub fn add_pool_size<'a>(
        &'a mut self,
        descriptor_type: vk::DescriptorType,
        count: u32,
    ) -> &'a mut LveDescriptorPoolBuilder {
        self.pool_sizes.push(vk::DescriptorPoolSize {
            ty: descriptor_type,
            descriptor_count: count,
        });

        self
    }

    pub fn set_pool_flags<'a>(
        &'a mut self,
        flags: vk::DescriptorPoolCreateFlags,
    ) -> &'a mut LveDescriptorPoolBuilder {
        self.pool_flags = flags;
        self
    }

    pub fn set_max_sets<'a>(&'a mut self, count: u32) -> &'a mut LveDescriptorPoolBuilder {
        self.max_sets = count;
        self
    }

    pub fn build(&self) -> Rc<LveDescriptorPool> {
        LveDescriptorPool::new(
            Rc::clone(&self.lve_device),
            self.max_sets,
            self.pool_flags,
            &self.pool_sizes,
        )
    }
}

pub struct LveDescriptorWriter {
    set_layout: Rc<LveDescriptorSetLayout>,
    pool: Rc<LveDescriptorPool>,
    writes: Vec<vk::WriteDescriptorSet>,
}

impl LveDescriptorWriter {
    pub fn new(
        set_layout: Rc<LveDescriptorSetLayout>,
        pool: Rc<LveDescriptorPool>,
    ) -> LveDescriptorWriter {
        LveDescriptorWriter {
            set_layout,
            pool,
            writes: Vec::<vk::WriteDescriptorSet>::new(),
        }
    }

    pub fn write_buffer<'a>(
        &'a mut self,
        binding: u32,
        buffer_info: &[vk::DescriptorBufferInfo],
    ) -> &'a mut LveDescriptorWriter {
        assert!(
            self.set_layout.bindings.contains_key(&binding),
            "Layout does not contain specified binding"
        );

        let binding_description = self.set_layout.bindings.get(&binding).unwrap();

        assert!(
            binding_description.descriptor_count == 1,
            "Binding single descriptor info, but binding expects multiple"
        );

        let write = vk::WriteDescriptorSet::builder()
            .descriptor_type(binding_description.descriptor_type)
            .dst_binding(binding)
            .buffer_info(buffer_info)
            .build();

        self.writes.push(write);

        self
    }

    pub fn write_image<'a>(
        &'a mut self,
        binding: u32,
        image_info: vk::DescriptorImageInfo,
    ) -> &'a mut LveDescriptorWriter {
        assert!(
            self.set_layout.bindings.contains_key(&binding),
            "Layout does not contain specified binding"
        );

        let binding_description = self.set_layout.bindings.get(&binding).unwrap();

        assert!(
            binding_description.descriptor_count == 1,
            "Binding single descriptor info, but binding expects multiple"
        );

        let write = vk::WriteDescriptorSet::builder()
            .descriptor_type(binding_description.descriptor_type)
            .dst_binding(binding)
            .image_info(&[image_info])
            .build();

        self.writes.push(write);

        self
    }

    pub fn build(&mut self) -> Result<vk::DescriptorSet, ()> {
        match self
            .pool
            .allocate_descriptor(self.set_layout.descriptor_set_layout)
        {
            Ok(set) => {
                unsafe { self.overwrite(&set) }
                Ok(set)
            }
            Err(_) => Err(()),
        }
    }
    pub unsafe fn overwrite(&mut self, set: &vk::DescriptorSet) {
        self.writes.iter_mut().for_each(|write| {
            write.dst_set = *set;
        });

        self.pool
            .lve_device
            .device
            .update_descriptor_sets(self.writes.as_slice(), &[])
    }
}
