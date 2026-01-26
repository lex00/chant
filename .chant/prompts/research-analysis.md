---
name: research-analysis
purpose: Analyze data or code and extract structured findings
---

# Research Analysis

You are performing research analysis for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Analysis Process

### 1. Understand the Input and Output Formats

**Origin files** (data to analyze):
- These are the source materials you will analyze systematically
- May include CSV, JSON, logs, configuration files, or other data formats
- Read all origin files completely before proceeding

**Informed by files** (context/reference):
- These provide context, schemas, or reference information
- Read these to understand the domain and constraints
- Use them to calibrate your analysis

**Target files** (output):
- Specify the format(s) you must produce:
  - **Markdown (.md)**: Structured with headers, tables, lists
  - **EDN (.edn)**: Clojure EDN syntax with descriptive comments
  - **JSON (.json)**: Valid JSON, pretty-printed with semantic structure

### 2. Read and Process All Input Files

1. **Read all origin files** - Process them completely and systematically
   - Don't sample or skip data
   - Extract all relevant information
   - Take detailed notes on patterns, anomalies, and findings

2. **Read all informed_by files** - Understand the context
   - Note schemas, types, and valid values
   - Understand constraints and business rules
   - Identify edge cases and special handling

3. **Systematically analyze** - Don't summarize, extract precisely
   - Extract exact values, not summaries
   - Capture all variants, not representative samples
   - Note completeness and data quality issues
   - Record frequency, distributions, and patterns

### 3. Apply Principles of Structured Analysis

- **Systematic** — Process all data methodically, don't skip sections
- **Precise** — Extract exact details, field names, types, values
- **Structured** — Organize output by logical categories or sections
- **Complete** — Don't abbreviate; capture every variant and edge case
- **Exhaustive** — Be thorough; err on the side of including too much rather than too little

### 4. Output Format Guidelines

#### Markdown Output (.md)

Structure with clear hierarchy:
```markdown
# Main Analysis Title

## Section 1: Category Name

### Subsection: Specific Finding

- **Item**: Description with specific data
- **Item**: Description with counts/examples

## Summary Statistics

| Metric | Value |
|--------|-------|
| Total items | X |
| Unique values | Y |

## Notable Patterns and Anomalies

- Pattern 1: Description with evidence
- Anomaly 1: Description with examples
```

#### EDN Output (.edn)

Use Clojure EDN syntax with descriptive comments:
```clojure
{:title "Analysis Results"
 :metadata {:created "2026-01-26"
            :source ["file1.csv" "file2.json"]
            :format "edn"}

 ;; Main findings organized by category
 :findings
 [{:category "Category 1"
   :items [{:name "Item 1"
            :value "specific value"
            :count 42}
           {:name "Item 2"
            :value "another value"
            :count 17}]}

  {:category "Category 2"
   :items [...]}]

 ;; Derived schema or structure (for Wobble use case)
 :schema
 [{:type "variant-name"
   :fields {:field1 "type1"
            :field2 "type2"}
   :example {:field1 "value1"
             :field2 "value2"}}]

 :summary {:total-items 100
           :unique-values 42
           :coverage "complete"}}
```

#### JSON Output (.json)

Valid JSON, pretty-printed with semantic structure:
```json
{
  "title": "Analysis Results",
  "metadata": {
    "created": "2026-01-26",
    "source": ["file1.csv", "file2.json"],
    "format": "json"
  },
  "findings": [
    {
      "category": "Category 1",
      "items": [
        {
          "name": "Item 1",
          "value": "specific value",
          "count": 42
        },
        {
          "name": "Item 2",
          "value": "another value",
          "count": 17
        }
      ]
    }
  ],
  "summary": {
    "totalItems": 100,
    "uniqueValues": 42,
    "coverage": "complete"
  }
}
```

### 5. For Schema Derivation (Wobble Use Case)

When analyzing code or specs to derive schema sections:

- **Be exhaustive** — Find and document every variant, enum value, or type combination
- **Be precise** — Use exact type names, not generic types like "any" or "object"
- **Include examples** — Provide concrete examples as comments or in example fields
- **Cross-reference** — Link related schema sections together
- **Note constraints** — Document validation rules, required fields, allowed values
- **Capture patterns** — Identify structural patterns and reusable components

Example for schema extraction:
```clojure
:schema
 [{:name "SpecType"
   :variants ["code" "task" "driver" "group" "research"]
   :description "The type of spec - code specs are for implementation..."
   :examples ["code" "task"]
   :used-in ["spec frontmatter" "config.rs"]}

  {:name "SpecStatus"
   :variants ["pending" "ready" "in_progress" "blocked" "completed"]
   :transitions
   {:pending ["ready" "blocked"]
    :ready ["in_progress" "blocked"]
    :in_progress ["completed" "blocked"]
    :blocked ["pending"]
    :completed []}
   :description "The current status of a spec..."}]
```

## Research Questions

If your spec includes specific research questions or findings to extract:

1. Read the research questions in your spec description
2. For each question, provide evidence-based answers with citations
3. Reference specific data points, line numbers, or file locations
4. If a question cannot be fully answered, note what data is missing

## Verification

Before producing final output:

1. **Completeness check**: Did you process every origin file?
2. **Format check**: Does output match target file format exactly?
3. **Accuracy check**: Can you cite the source for each finding?
4. **Exhaustiveness check**: Did you capture all variants, not just common ones?
5. **Verification against acceptance criteria**: Does your output meet all stated requirements?

## Constraints

- Read all files completely; don't sample or skip
- Extract precise data, not summaries
- Output exactly in the specified format(s)
- For EDN/JSON: ensure valid syntax
- For Markdown: use clear structure and organization
- Don't omit edge cases or unusual variants
- Include evidence citations for significant findings

## Acceptance Criteria

- [ ] All origin files fully read and processed
- [ ] All informed_by files read for context
- [ ] Output file(s) created in correct format(s)
- [ ] Output is precise and not summarized
- [ ] All findings are exhaustive, not representative
- [ ] Output syntax is valid (EDN/JSON/Markdown)
- [ ] All acceptance criteria from spec met
- [ ] Commit with message: `chant({{spec.id}}): <analysis summary>`
