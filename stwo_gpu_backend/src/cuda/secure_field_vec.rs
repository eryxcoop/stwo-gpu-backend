use stwo_prover::core::fields::qm31::SecureField;

use super::bindings;

#[derive(Clone, Debug)]
pub struct SecureFieldVec {
    pub(crate) device_ptr: *const u32,
    pub(crate) size: usize,
}

impl SecureFieldVec {
    pub fn new(device_ptr: *const u32, size: usize) -> Self {
        Self { device_ptr, size }
    }
    pub fn from_vec(host_array: Vec<SecureField>) -> Self {
        let device_ptr = unsafe {
            bindings::copy_uint32_t_vec_from_host_to_device(
                host_array.as_ptr() as *const u32,
                4 * host_array.len() as u32,
            )
        };
        let size = host_array.len();
        Self::new(device_ptr, size)
    }

    pub fn to_vec(&self) -> Vec<SecureField> {
        let mut host_data: Vec<SecureField> = Vec::with_capacity(self.size);
        unsafe {
            host_data.set_len(self.size.try_into().unwrap());
            bindings::copy_uint32_t_vec_from_device_to_host(
                self.device_ptr,
                host_data.as_mut_ptr() as *const u32,
                4 * self.size as u32,
            );
        }
        host_data
    }
}

impl Drop for SecureFieldVec {
    fn drop(&mut self) {
        unsafe { bindings::free_uint32_t_vec(self.device_ptr) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stwo_prover::core::fields::qm31::SecureField;

    #[test]
    fn test_constructor() {
        let size = 1 << 5;
        let from_raw = (1..(size + 1) as u32).collect::<Vec<u32>>();
        let host_data = from_raw
            .chunks(4)
            .map(|a| SecureField::from_u32_unchecked(a[0], a[1], a[2], a[3]))
            .collect::<Vec<_>>();
        let secure_field_vec = SecureFieldVec::from_vec(host_data.clone());

        assert_eq!(secure_field_vec.to_vec(), host_data);
        assert_eq!(secure_field_vec.size, host_data.len());
    }
}
