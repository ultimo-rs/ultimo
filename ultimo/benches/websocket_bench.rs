//! Benchmark tests for WebSocket frame codec performance

#![cfg(feature = "websocket")]

use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ultimo::websocket::test_helpers::{Frame, OpCode};

/// Benchmark frame encoding with different payload sizes
fn bench_frame_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_encode");

    for size in [0, 125, 126, 1024, 4096, 65535, 65536].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let payload = Bytes::from(vec![0u8; size]);
            let frame = Frame {
                fin: true,
                opcode: OpCode::Binary,
                mask: None,
                payload,
            };

            b.iter(|| black_box(frame.encode()));
        });
    }

    group.finish();
}

/// Benchmark frame encoding with masking
fn bench_frame_encode_masked(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_encode_masked");

    for size in [125, 1024, 4096, 65536].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let payload = Bytes::from(vec![0xABu8; size]);
            let frame = Frame {
                fin: true,
                opcode: OpCode::Binary,
                mask: Some([0x12, 0x34, 0x56, 0x78]),
                payload,
            };

            b.iter(|| black_box(frame.encode()));
        });
    }

    group.finish();
}

/// Benchmark frame decoding with different payload sizes
fn bench_frame_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_decode");

    for size in [0, 125, 126, 1024, 4096, 65535, 65536].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let payload = Bytes::from(vec![0u8; size]);
            let frame = Frame {
                fin: true,
                opcode: OpCode::Binary,
                mask: None,
                payload,
            };
            let encoded = frame.encode();

            b.iter(|| {
                let mut buf = BytesMut::from(encoded.as_ref());
                black_box(Frame::parse(&mut buf))
            });
        });
    }

    group.finish();
}

/// Benchmark frame decoding with masked frames
fn bench_frame_decode_masked(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_decode_masked");

    for size in [125, 1024, 4096, 65536].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let payload = Bytes::from(vec![0xABu8; size]);
            let frame = Frame {
                fin: true,
                opcode: OpCode::Binary,
                mask: Some([0x12, 0x34, 0x56, 0x78]),
                payload,
            };
            let encoded = frame.encode();

            b.iter(|| {
                let mut buf = BytesMut::from(encoded.as_ref());
                black_box(Frame::parse(&mut buf))
            });
        });
    }

    group.finish();
}

/// Benchmark round-trip (encode + decode)
fn bench_round_trip(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_trip");

    for size in [125, 1024, 4096, 65536].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let payload = Bytes::from(vec![0xABu8; size]);
            let frame = Frame {
                fin: true,
                opcode: OpCode::Binary,
                mask: Some([0x12, 0x34, 0x56, 0x78]),
                payload,
            };

            b.iter(|| {
                let encoded = frame.encode();
                let mut buf = BytesMut::from(encoded.as_ref());
                black_box(Frame::parse(&mut buf))
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_frame_encode,
    bench_frame_encode_masked,
    bench_frame_decode,
    bench_frame_decode_masked,
    bench_round_trip,
);

criterion_main!(benches);
