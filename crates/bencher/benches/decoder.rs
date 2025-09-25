use std::hint::black_box;
use bencher::{TestCase, TestFile};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use micro_http::codec::RequestDecoder;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Decoder;

static SMALL_HEADER: TestFile = TestFile::new("get_small.txt", include_str!("../resources/request/get_small.txt"));
static LARGE_HEADER: TestFile = TestFile::new("get_large.txt", include_str!("../resources/request/get_large.txt"));

fn create_test_cases() -> Vec<TestCase> {
    vec![TestCase::normal("small_header_decoder", SMALL_HEADER), TestCase::normal("large_header_decoder", LARGE_HEADER)]
}

fn benchmark_request_decoder(criterion: &mut Criterion) {
    let test_cases = create_test_cases();
    let mut group = criterion.benchmark_group("request_decoder");

    for case in test_cases {
        group.throughput(Throughput::Bytes(case.file().content().len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(case.name()), &case, |b, case| {
            let mut request_decoder = RequestDecoder::new();
            b.iter_batched_ref(
                || BytesMut::from(case.file().content()),
                |bytes_mut| {
                    let header = request_decoder.decode(bytes_mut).expect("input should be valid http request header").unwrap();
                    let body = request_decoder.decode(bytes_mut).expect("input should be valid http request body").unwrap();
                    black_box((header, body));
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(decoder, benchmark_request_decoder);
criterion_main!(decoder);
