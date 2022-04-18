use super::lve_device::LveDevice;

use ash::vk;

use std::{ffi::c_void, ptr, rc::Rc};

pub enum BufferType {
    Staging,
    Vertex,
    Index,
    Uniform,
}

pub struct LveBuffer {
    lve_device: Rc<LveDevice>,
    pub buffer: vk::Buffer,
    pub buffer_size: vk::DeviceSize,
    pub memory: vk::DeviceMemory,
    pub memory_property_flags: vk::MemoryPropertyFlags,
    pub mapped: *mut c_void,
    pub instance_count: u32,
    pub instance_size: vk::DeviceSize,
    pub alignment_size: vk::DeviceSize,
    pub usage_flags: vk::BufferUsageFlags,
    buffer_type: BufferType,
}

impl LveBuffer {
    pub fn new(
        device: Rc<LveDevice>,
        instance_size: vk::DeviceSize,
        instance_count: u32,
        usage_flags: vk::BufferUsageFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
        min_offset_alignment: vk::DeviceSize, // Should be 1 for vertex and index buffers
        buffer_type: BufferType,
    ) -> LveBuffer {
        let alignment_size = LveBuffer::get_alignment(instance_size, min_offset_alignment);
        let buffer_size = alignment_size * instance_count as u64;
        let (buffer, memory) =
            device.create_buffer(buffer_size, usage_flags, memory_property_flags);

        LveBuffer {
            lve_device: device,
            buffer: buffer,
            buffer_size,
            memory,
            memory_property_flags,
            mapped: ptr::null_mut(),
            instance_count,
            instance_size,
            alignment_size,
            usage_flags,
            buffer_type,
        }
    }

    /**
     * Map a memory range of this buffer. If successful, mapped points to the specified buffer range.
     *
     * @param size (Optional) Size of the memory range to map. Pass VK_WHOLE_SIZE to map the complete
     * buffer range.
     * @param offset (Optional) Byte offset from beginning
     *
     * @return VkResult of the buffer mapping call
     */
    pub unsafe fn map(&mut self, size: vk::DeviceSize, offset: vk::DeviceSize) {
        // Don't need the assert as this can only be called after the creation of the buffer
        self.mapped = self
            .lve_device
            .device
            .map_memory(self.memory, offset, size, vk::MemoryMapFlags::empty())
            .map_err(|e| log::error!("Failed to map buffer memory: {}", e))
            .unwrap();
    }

    /**
     * Unmap a mapped memory range
     *
     * @note Does not return a result as vkUnmapMemory can't fail
     */
    pub unsafe fn unmap(&mut self) {
        if !self.mapped.is_null() {
            self.lve_device.device.unmap_memory(self.memory);
            self.mapped = ptr::null_mut();
        }
    }

    /**
     * Copies the specified data to the mapped buffer. Default value writes whole buffer range
     *
     * @param data Pointer to the data to copy
     * @param size (Optional) Size of the data to copy. Pass VK_WHOLE_SIZE to flush the complete buffer
     * range.
     * @param offset (Optional) Byte offset from beginning of mapped region
     *
     */
    pub unsafe fn write_to_buffer<T: Copy>(
        &self,
        data: &[T],
        size: vk::DeviceSize,
        offset: vk::DeviceSize,
    ) {
        assert!(!self.mapped.is_null(), "Cannot copy to unmapped buffer");

        if size == vk::WHOLE_SIZE {
            let mut align = ash::util::Align::new(
                self.mapped,
                std::mem::align_of::<u32>() as u64,
                self.buffer_size,
            );
            align.copy_from_slice(data);
        } else {
            let mem_offset = self.mapped.add(offset as usize);
            let mut align =
                ash::util::Align::new(mem_offset, std::mem::align_of::<u32>() as u64, size);
            align.copy_from_slice(data);
        }
    }

    /**
     * Flush a memory range of the buffer to make it visible to the device
     *
     * @note Only required for non-coherent memory
     *
     * @param size (Optional) Size of the memory range to flush. Pass VK_WHOLE_SIZE to flush the
     * complete buffer range.
     * @param offset (Optional) Byte offset from beginning
     *
     * @return VkResult of the flush call
     */
    pub unsafe fn flush(
        &self,
        size: vk::DeviceSize,
        offset: vk::DeviceSize,
    ) -> Result<(), vk::Result> {
        let mapped_range = vk::MappedMemoryRange::builder()
            .memory(self.memory)
            .size(size)
            .offset(offset)
            .build();

        let ranges = [mapped_range];

        self.lve_device.device.flush_mapped_memory_ranges(&ranges)
    }

    /**
     * Invalidate a memory range of the buffer to make it visible to the host
     *
     * @note Only required for non-coherent memory
     *
     * @param size (Optional) Size of the memory range to invalidate. Pass VK_WHOLE_SIZE to invalidate
     * the complete buffer range.
     * @param offset (Optional) Byte offset from beginning
     *
     * @return VkResult of the invalidate call
     */
    pub unsafe fn invalidate(
        &self,
        size: vk::DeviceSize,
        offset: vk::DeviceSize,
    ) -> Result<(), vk::Result> {
        let mapped_range = vk::MappedMemoryRange::builder()
            .memory(self.memory)
            .size(size)
            .offset(offset)
            .build();

        let ranges = [mapped_range];

        self.lve_device
            .device
            .invalidate_mapped_memory_ranges(&ranges)
    }

    /**
     * Create a buffer info descriptor
     *
     * @param size (Optional) Size of the memory range of the descriptor
     * @param offset (Optional) Byte offset from beginning
     *
     * @return VkDescriptorBufferInfo of specified offset and range
     */
    pub fn descriptor_info(
        &self,
        size: vk::DeviceSize,
        offset: vk::DeviceSize,
    ) -> vk::DescriptorBufferInfo {
        vk::DescriptorBufferInfo {
            buffer: self.buffer,
            offset,
            range: size,
        }
    }

    /**
     * Copies "instanceSize" bytes of data to the mapped buffer at an offset of index * alignmentSize
     *
     * @param data Pointer to the data to copy
     * @param index Used in offset calculation
     *
     */
    pub unsafe fn write_to_index<T: Copy>(&self, data: &[T], index: u64) {
        self.write_to_buffer(data, self.instance_size, index * self.alignment_size)
    }

    /**
     *  Flush the memory range at index * alignmentSize of the buffer to make it visible to the device
     *
     * @param index Used in offset calculation
     *
     */
    pub unsafe fn flush_index(&self, index: u64) -> Result<(), vk::Result> {
        self.flush(self.alignment_size, index * self.alignment_size)
    }

    /**
     * Create a buffer info descriptor
     *
     * @param index Specifies the region given by index * alignmentSize
     *
     * @return VkDescriptorBufferInfo for instance at index
     */
    pub fn descriptor_info_for_index(&self, index: u64) -> vk::DescriptorBufferInfo {
        self.descriptor_info(self.alignment_size, index * self.alignment_size)
    }

    /**
     * Invalidate a memory range of the buffer to make it visible to the host
     *
     * @note Only required for non-coherent memory
     *
     * @param index Specifies the region to invalidate: index * alignmentSize
     *
     * @return VkResult of the invalidate call
     */
    pub unsafe fn invalidate_index(&self, index: u64) -> Result<(), vk::Result> {
        self.invalidate(self.alignment_size, index * self.alignment_size)
    }

    fn get_alignment(
        instance_size: vk::DeviceSize,
        min_offset_alignment: vk::DeviceSize,
    ) -> vk::DeviceSize {
        if min_offset_alignment > 0 {
            return (instance_size + min_offset_alignment - 1) & !(min_offset_alignment - 1);
        }

        instance_size
    }
}

impl Drop for LveBuffer {
    fn drop(&mut self) {
        match &self.buffer_type {
            BufferType::Staging => log::debug!("Dropping Staging Buffer"),
            BufferType::Vertex => log::debug!("Dropping Vertex Buffer"),
            BufferType::Index => log::debug!("Dropping Index Buffer"),
            BufferType::Uniform => log::debug!("Dropping Uniform Buffer"),
        }

        unsafe {
            self.unmap();
            self.lve_device.device.destroy_buffer(self.buffer, None);
            self.lve_device.device.free_memory(self.memory, None);
        }
    }
}
