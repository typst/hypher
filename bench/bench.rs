use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hyphenation::{Hyphenator, Load};

fn criterion_benchmark(c: &mut Criterion) {
    let english = "extensive";
    let greek = "διαμερίσματα";

    bench(c, "hypher-english", || {
        drain(hypher::hyphenate(black_box(english), black_box(hypher::Lang::English)))
    });

    bench(c, "hypher-greek", || {
        drain(hypher::hyphenate(black_box(greek), black_box(hypher::Lang::Greek)))
    });

    let standard_english =
        hyphenation::Standard::from_embedded(hyphenation::Language::EnglishUS).unwrap();

    let standard_greek =
        hyphenation::Standard::from_embedded(hyphenation::Language::GreekMono).unwrap();

    bench(c, "hyphenation-english", || {
        drain(black_box(&standard_english).hyphenate(black_box(english)).breaks)
    });

    bench(c, "hyphenation-greek", || {
        drain(black_box(&standard_greek).hyphenate(black_box(greek)).breaks)
    });

    bench(c, "hyphenation-load-english", || {
        hyphenation::Standard::from_embedded(black_box(hyphenation::Language::EnglishUS))
    });

    bench(c, "hyphenation-load-greek", || {
        hyphenation::Standard::from_embedded(black_box(hyphenation::Language::GreekMono))
    });
}

fn bench<R>(c: &mut Criterion, name: &str, f: impl FnMut() -> R + Copy) {
    c.bench_function(name, |b| b.iter(f));
}

fn drain<T>(iter: impl IntoIterator<Item = T>) {
    for _ in iter {}
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
