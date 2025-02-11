use bencher::{TestCase, TestFile};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use micro_http::codec::RequestDecoder;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Decoder;

static SMALL_HEADER: TestFile = TestFile::new("get_small.txt", include_str!("../resources/request/get_small.txt"));
static LARGE_HEADER: TestFile = TestFile::new("get_large.txt", include_str!("../resources/request/get_large.txt"));

fn create_test_cases() -> Vec<TestCase> {
    vec![
        TestCase::normal("small_header_decoder", SMALL_HEADER.clone()),
        TestCase::normal("large_header_decoder", LARGE_HEADER.clone())
    ]
}

fn benchmark_request_decoder(criterion: &mut Criterion) {
    let test_cases = create_test_cases();
    let mut group = criterion.benchmark_group("request_decoder");

    for case in test_cases {
        group.throughput(Throughput::Bytes(case.file().content().len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(case.name()), &case, |b, case| {
            b.iter_batched_ref(
                || (RequestDecoder::new(), BytesMut::from(case.file().content())),
                |(decoder, bytes_mut)| {
                    let header = decoder.decode(bytes_mut).expect("input should be valide http request header").unwrap();
                    black_box(header);
                    let body = decoder.decode(bytes_mut).expect("input should be valide http request body").unwrap();
                    black_box(body);
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(decoder, benchmark_request_decoder);
criterion_main!(decoder);
