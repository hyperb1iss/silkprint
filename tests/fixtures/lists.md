---
title: List Showcase
author: SilkPrint Test Suite
---

# Lists

## Deeply Nested Unordered List

- Level 1 item A
  - Level 2 item A.1
    - Level 3 item A.1.1
      - Level 4 item A.1.1.1
        - Level 5 item A.1.1.1.1
      - Level 4 item A.1.1.2
    - Level 3 item A.1.2
  - Level 2 item A.2
- Level 1 item B
  - Level 2 item B.1
  - Level 2 item B.2
    - Level 3 item B.2.1
- Level 1 item C

## Ordered Lists with Custom Start

1. First item
2. Second item
3. Third item

5. Starting from five
6. Continuing from six
7. Seven

100. Large starting number
101. Next
102. And next

## Task Lists

- [x] Set up project structure
- [x] Implement Markdown parser
- [x] Build theme engine
- [ ] Write comprehensive tests
- [ ] Publish to crates.io
- [x] Add font embedding
- [ ] HTML output engine

## Description Lists

Term 1
: Definition for term 1.

Term 2
: First definition for term 2.
: Second definition for term 2.

A Longer Term Name
: This definition is a full paragraph that explains the longer term in more
  detail. It may wrap across multiple lines in the output.

## Mixed Nested Lists

1. First ordered item
   - Unordered child A
   - Unordered child B
     1. Sub-ordered 1
     2. Sub-ordered 2
        - Deep unordered
          - [x] Deep task checked
          - [ ] Deep task unchecked
   - Unordered child C
2. Second ordered item
   - [x] Task under ordered
   - [ ] Another task
3. Third ordered item

## Lists with Rich Content

- **Bold list item** with some regular text after it
- *Italic list item* that also has `inline code` in it
- List item with a [link](https://example.com) and ~~strikethrough~~
- List item containing a long paragraph that should wrap across multiple
  lines when rendered at the standard page width. This tests that list
  indentation is maintained correctly across line breaks.

## Single Item Lists

- Just one bullet

1. Just one numbered item

- [x] Just one task
