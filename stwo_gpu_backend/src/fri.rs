use stwo_prover::core::{
    fields::qm31::SecureField,
    fri::FriOps,
    poly::{circle::SecureEvaluation, line::LineEvaluation, twiddles::TwiddleTree},
};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::secure_column::SecureColumn;

use crate::backend::CudaBackend;
use crate::cuda::{BaseFieldVec, bindings};

impl FriOps for CudaBackend {
    fn fold_line(
        eval: &LineEvaluation<Self>,
        alpha: SecureField,
        twiddles: &TwiddleTree<Self>,
    ) -> LineEvaluation<Self> {
        unsafe {
            let n = eval.len();
            assert!(n >= 2, "Evaluation too small");

            let remaining_folds = n.ilog2();
            let twiddles_size = twiddles.itwiddles.size;
            let twiddle_offset: usize = twiddles_size - (1 << remaining_folds);

            let folded_values = alloc_secure_column_on_gpu_as_array(n >> 1);

            launch_kernel_for_fold(
                &eval.values,
                twiddles, twiddle_offset,
                [
                    &folded_values[0],
                    &folded_values[1],
                    &folded_values[2],
                    &folded_values[3]
                ],
                alpha,
                n);

            let folded_values = SecureColumn { columns: folded_values };
            LineEvaluation::new(eval.domain().double(), folded_values)
        }
    }

    fn fold_circle_into_line(
        _dst: &mut LineEvaluation<Self>,
        _src: &SecureEvaluation<Self>,
        _alpha: SecureField,
        _twiddles: &TwiddleTree<Self>,
    ) {
        todo!()
    }

    fn decompose(eval: &SecureEvaluation<Self>) -> (SecureEvaluation<Self>, SecureField) {
        let columns = &eval.columns;

        let lambda = unsafe {
            let a: M31 = Self::sum(&columns[0]);
            let b = Self::sum(&columns[1]);
            let c = Self::sum(&columns[2]);
            let d = Self::sum(&columns[3]);
            SecureField::from_m31(a, b, c, d) / M31::from_u32_unchecked(eval.len() as u32)
        };

        let g_values = unsafe {
            SecureColumn {
                columns: [
                    Self::compute_g_values(&columns[0], lambda.0.0),
                    Self::compute_g_values(&columns[1], lambda.0.1),
                    Self::compute_g_values(&columns[2], lambda.1.0),
                    Self::compute_g_values(&columns[3], lambda.1.1),
                ]
            }
        };

        let g = SecureEvaluation {
            domain: eval.domain,
            values: g_values,
        };
        (g, lambda)
    }
}

unsafe fn launch_kernel_for_fold(
    eval_values: &SecureColumn<CudaBackend>,
    twiddles: &TwiddleTree<CudaBackend>,
    twiddle_offset: usize,
    folded_values: [&BaseFieldVec; 4],
    alpha: SecureField,
    n: usize) {
    let gpu_domain = twiddles.itwiddles.device_ptr;

    bindings::fold_circle(gpu_domain, twiddle_offset, n,
                          eval_values.columns[0].device_ptr,
                          eval_values.columns[1].device_ptr,
                          eval_values.columns[2].device_ptr,
                          eval_values.columns[3].device_ptr,
                          alpha,
                          folded_values[0].device_ptr,
                          folded_values[1].device_ptr,
                          folded_values[2].device_ptr,
                          folded_values[3].device_ptr,
    );
}

unsafe fn alloc_secure_column_on_gpu_as_array(n: usize) -> [BaseFieldVec; 4] {
    let folded_values_0 = BaseFieldVec::new_zeroes(n);
    let folded_values_1 = BaseFieldVec::new_zeroes(n);
    let folded_values_2 = BaseFieldVec::new_zeroes(n);
    let folded_values_3 = BaseFieldVec::new_zeroes(n);

    [folded_values_0, folded_values_1, folded_values_2, folded_values_3]
}

impl CudaBackend {
    unsafe fn sum(column: &BaseFieldVec) -> BaseField {
        let column_size = column.size;
        return bindings::sum(column.device_ptr,
                             column_size as u32);
    }

    unsafe fn compute_g_values(f_values: &BaseFieldVec, lambda: M31) -> BaseFieldVec {
        let size = f_values.size;

        let result = BaseFieldVec {
            device_ptr: bindings::compute_g_values(
                f_values.device_ptr,
                size,
                lambda),
            size: size,
        };
        return result;
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use rand::{Rng, SeedableRng};
    use rand::rngs::SmallRng;
    use stwo_prover::core::backend::{Column, CpuBackend};
    use stwo_prover::core::fields::m31::{BaseField, M31};
    use stwo_prover::core::fields::qm31::SecureField;
    use stwo_prover::core::fields::secure_column::SecureColumn;
    use stwo_prover::core::fri::FriOps;
    use stwo_prover::core::poly::circle::{CanonicCoset, PolyOps, SecureEvaluation};
    use stwo_prover::core::poly::line::{LineDomain, LineEvaluation};

    use crate::backend::CudaBackend;
    use crate::cuda::BaseFieldVec;

    fn test_decompose_with_domain_log_size(domain_log_size: u32) {
        let size = 1 << domain_log_size;
        let coset = CanonicCoset::new(domain_log_size);
        let domain = coset.circle_domain();

        let from_raw = (0..size * 4 as u32).collect::<Vec<u32>>();
        let mut vec: [Vec<M31>; 4] = [vec!(), vec!(), vec!(), vec!()];

        from_raw
            .chunks(4)
            .for_each(|a| {
                vec[0].push(M31::from_u32_unchecked(a[0]));
                vec[1].push(M31::from_u32_unchecked(a[1]));
                vec[2].push(M31::from_u32_unchecked(a[2]));
                vec[3].push(M31::from_u32_unchecked(a[3]));
            });

        let cpu_secure_evaluation = SecureEvaluation {
            domain: domain,
            values: SecureColumn { columns: vec.clone() },
        };

        let columns = [
            BaseFieldVec::from_vec(vec[0].clone()),
            BaseFieldVec::from_vec(vec[1].clone()),
            BaseFieldVec::from_vec(vec[2].clone()),
            BaseFieldVec::from_vec(vec[3].clone())];
        let gpu_secure_evaluation = SecureEvaluation {
            domain: domain,
            values: SecureColumn { columns },
        };

        let (expected_g_values, expected_lambda) = CpuBackend::decompose(&cpu_secure_evaluation);
        let (g_values, lambda) = CudaBackend::decompose(&gpu_secure_evaluation);

        assert_eq!(lambda, expected_lambda);
        assert_eq!(g_values.values.columns[0].to_cpu(), expected_g_values.values.columns[0]);
        assert_eq!(g_values.values.columns[1].to_cpu(), expected_g_values.values.columns[1]);
        assert_eq!(g_values.values.columns[2].to_cpu(), expected_g_values.values.columns[2]);
        assert_eq!(g_values.values.columns[3].to_cpu(), expected_g_values.values.columns[3]);
    }

    #[test]
    fn test_decompose_using_less_than_an_entire_block() {
        test_decompose_with_domain_log_size(5);
    }

    #[test]
    fn test_decompose_using_an_entire_block() {
        test_decompose_with_domain_log_size(11);
    }

    #[test]
    fn test_decompose_using_more_than_entire_block() {
        test_decompose_with_domain_log_size(11 + 4);
    }

    #[test]
    fn test_decompose_using_an_entire_block_for_results() {
        test_decompose_with_domain_log_size(22);
    }

    #[ignore]
    #[test]
    fn test_decompose_using_more_than_an_entire_block_for_results() {
        test_decompose_with_domain_log_size(27);
    }

    #[test]
    fn test_fold_line_compared_with_cpu() {
        const LOG_SIZE: u32 = 20;
        let mut rng = SmallRng::seed_from_u64(0);
        let values: Vec<SecureField> = (0..1 << LOG_SIZE).map(|_| rng.gen()).collect_vec();
        let alpha = SecureField::from_u32_unchecked(1, 3, 5, 7);
        let domain = LineDomain::new(CanonicCoset::new(LOG_SIZE + 1).half_coset());

        let mut vec: [Vec<BaseField>; 4] = [vec!(), vec!(), vec!(), vec!()];
        values.iter()
            .for_each(|a| {
                vec[0].push(BaseField::from_u32_unchecked(a.0.0.0));
                vec[1].push(BaseField::from_u32_unchecked(a.0.1.0));
                vec[2].push(BaseField::from_u32_unchecked(a.1.0.0));
                vec[3].push(BaseField::from_u32_unchecked(a.1.1.0));
            });

        let cpu_fold = CpuBackend::fold_line(
            &LineEvaluation::new(domain, SecureColumn {columns: vec.clone()}),
            alpha,
            &CpuBackend::precompute_twiddles(domain.coset()),
        );
        let vecs = [
            BaseFieldVec::from_vec(vec[0].clone()),
            BaseFieldVec::from_vec(vec[1].clone()),
            BaseFieldVec::from_vec(vec[2].clone()),
            BaseFieldVec::from_vec(vec[3].clone())];

        let gpu_fold = CudaBackend::fold_line(
            &LineEvaluation::new(domain, SecureColumn { columns: vecs }),
            alpha,
            &CudaBackend::precompute_twiddles(domain.coset()),
        );

        assert_eq!(cpu_fold.values.to_vec(), gpu_fold.values.to_cpu().to_vec());
    }
}