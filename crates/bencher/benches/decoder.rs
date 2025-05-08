use bencher::{TestCase, TestFile};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use micro_http::codec::{PicoHeaderDecoder, RequestDecoder};
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Decoder;

static SMALL_HEADER: TestFile = TestFile::new("get_small.txt", include_str!("../resources/request/get_small.txt"));
static LARGE_HEADER: TestFile = TestFile::new("get_large.txt", include_str!("../resources/request/get_large.txt"));

fn create_test_cases() -> Vec<TestCase> {
    vec![TestCase::normal("small_header_decoder", SMALL_HEADER.clone()), TestCase::normal("large_header_decoder", LARGE_HEADER.clone())]
}

fn benchmark_request_decoder(criterion: &mut Criterion) {
    let test_cases = create_test_cases();
    let mut group = criterion.benchmark_group("request_decoder");

    for case in test_cases {
        group.throughput(Throughput::Bytes(case.file().content().len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(case.name()), &case, |b, case| {
            b.iter_batched_ref(
                ||  {
                    (BytesMut::from(case.file().content()), RequestDecoder::new())
                },
                |(bytes_mut, request_decoder)| {
                    let header = request_decoder.decode(bytes_mut).expect("input should be valid http request header").unwrap();
                    black_box(header);
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}


fn benchmark_pico_request_decoder(criterion: &mut Criterion) {
    let test_cases = create_test_cases();
    let mut group = criterion.benchmark_group("pico_request_decoder");
    for case in test_cases {
        group.throughput(Throughput::Bytes(case.file().content().len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(case.name()), &case, |b, case| {
            b.iter_batched_ref(
                ||  {
                    (BytesMut::from(case.file().content()), PicoHeaderDecoder::new())

                },
                |(bytes_mut, decoder)| {
                    let header = decoder.decode(bytes_mut).expect("input should be valid http request header").unwrap();
                    black_box(header);
                },
                BatchSize::SmallInput,
            );
        });
    }
}

criterion_group!(decoder, benchmark_pico_request_decoder);
criterion_main!(decoder);
