---
id: 003-rejected-change
title: Migrate all database queries to raw SQL
created: 2026-02-01T14:00:00Z
status: pending
approval:
  required: true
  status: rejected
  by: bob
  at: 2026-02-01T14:45:12Z
---

# Migrate all database queries to raw SQL

## Goal

Replace the current ORM (Sequelize) with raw SQL queries for better performance and control over database operations.

## Context

The ORM adds overhead and sometimes generates inefficient queries. Moving to raw SQL would give us complete control over query optimization and reduce the abstraction layer between our code and the database.

## Acceptance Criteria

- [ ] Remove Sequelize dependency
- [ ] Rewrite all model queries using raw SQL
- [ ] Create query builder utilities for common patterns
- [ ] Update all tests to work with raw SQL
- [ ] Add SQL injection protection utilities
- [ ] Migrate all database migrations from ORM to raw SQL
- [ ] Update database connection handling

## Performance Goals

Target metrics:
- 50% reduction in query execution time
- 30% reduction in memory usage
- Elimination of N+1 query problems

## Approval Discussion

**bob** - 2026-02-01 14:45 - REJECTED

This spec has several issues that need to be addressed before approval:

1. **Scope is too large**: Migrating the entire application at once is risky. This should be broken into incremental changes, starting with the most problematic queries.

2. **Missing risk assessment**: No rollback plan if performance doesn't improve or if we encounter issues mid-migration.

3. **SQL injection concerns**: The spec mentions "protection utilities" but doesn't specify which library or approach. We need a concrete plan for parameterized queries.

4. **Testing strategy unclear**: How do we ensure functional parity during migration? Need acceptance criteria for comprehensive integration testing.

5. **Team expertise**: Not everyone on the team is comfortable with raw SQL. Need training plan or consider a query builder library instead.

**Recommendation**: Split this into:
- Spec A: Profile and identify the top 10 slowest queries
- Spec B: Optimize those specific queries (can use raw SQL if needed)
- Spec C: Evaluate query builder libraries (Knex.js) as middle ground
- Then reassess whether full ORM removal is necessary

Do not proceed with this spec until it's restructured with a more incremental, safer approach.
