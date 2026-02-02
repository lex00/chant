# Efficient Transformers Through Sparse Attention

**Authors:** Smith, J., Chen, L., & Kumar, R.
**Published:** 2025
**Journal:** Neural Information Processing Systems (NeurIPS)

## Abstract

We present a novel sparse attention mechanism that reduces computational complexity from O(n²) to O(n log n) while maintaining 98% of full attention performance on standard benchmarks.

## Key Findings

### Approach

Our method uses learned sparse patterns instead of fixed patterns. The model learns which attention connections are most important during training and prunes less important connections.

### Results

- **Inference Speed:** 2.3x faster than dense attention
- **Memory Usage:** 60% reduction in peak memory
- **Accuracy:** Only 2% degradation on GLUE benchmark
- **Training Time:** 1.8x faster convergence

### Trade-offs

- Requires additional training phase to learn sparsity patterns
- Performance degrades on tasks requiring global context (e.g., long-document summarization)
- Works best for sequence lengths < 2048 tokens

## Methodology

We evaluated on:
- BERT-base architecture (110M parameters)
- GLUE benchmark tasks
- WikiText-103 perplexity
- SQuAD 2.0 question answering

## Limitations

- Not tested on encoder-decoder architectures
- Sparsity patterns are task-specific
- Requires retraining for new domains
