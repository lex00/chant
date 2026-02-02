# Low-Rank Adaptation for Transformer Efficiency

**Authors:** Jones, M., Patel, S., & Williams, A.
**Published:** 2024
**Conference:** International Conference on Machine Learning (ICML)

## Abstract

We introduce LoRA (Low-Rank Adaptation), which decomposes weight matrices into low-rank representations, reducing parameter counts by 90% while maintaining model performance.

## Key Findings

### Approach

Instead of fine-tuning all parameters, LoRA freezes pre-trained weights and injects trainable low-rank matrices into each layer. This dramatically reduces trainable parameters and memory footprint.

### Results

- **Parameter Reduction:** 90% fewer trainable parameters
- **Memory Usage:** 3x reduction during fine-tuning
- **Accuracy:** Matches full fine-tuning on 8/10 tasks
- **Fine-tuning Time:** 4x faster

### Trade-offs

- Slightly worse on tasks requiring significant distribution shift
- Requires careful rank selection (typically r=8 to r=64)
- Not beneficial for training from scratch

## Methodology

We evaluated on:
- GPT-3 175B (fine-tuning experiments)
- RoBERTa-large
- T5-base
- Multiple NLU and NLG tasks

## Key Insight

The success of LoRA suggests that fine-tuning updates have low intrinsic dimensionality. Most of the adaptation happens in a low-rank subspace.

## Limitations

- Only evaluated on natural language tasks
- Performance depends on choosing appropriate rank
- May not work for all layer types (e.g., LayerNorm)
