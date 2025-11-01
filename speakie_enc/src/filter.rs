use iir_filters::{
    filter::{DirectForm2Transposed, Filter},
    filter_design::butter,
    sos::zpk2sos,
};

/// Lowpass filter the signal to clean it for pitch detection
pub fn lowpass(inp: &[f64]) -> Vec<f64> {
    let zpk = butter(
        5,
        iir_filters::filter_design::FilterType::LowPass(800.),
        8000.,
    )
    .unwrap();
    let mut dft2 = DirectForm2Transposed::new(&zpk2sos(&zpk, None).unwrap());
    inp.iter().map(|x| dft2.filter(*x)).collect()
}
