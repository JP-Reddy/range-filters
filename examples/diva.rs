use range_filters::data_gen::generate_smooth_u16;
use range_filters::diva::Diva;

fn main() {
    let mut keys = generate_smooth_u16(Some(3000));
    keys.sort();
    let keys = keys.into_iter().map(|k| k as u64).collect::<Vec<_>>();

    println!("keys: {:?}", keys);
    let diva = Diva::new_with_keys(&keys, 1024, 0.01);

    println!("{}", diva);
}
