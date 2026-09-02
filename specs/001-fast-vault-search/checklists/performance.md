# Performance Checklist: Fast Vault Search

- [x] AI and embedding imports are absent from capture, board, and structural search paths.
- [x] Changed file metadata avoids reparsing unchanged notes.
- [x] 1,000-note structural index benchmark is under 3 seconds (latest: 1,002.13 ms).
- [x] Search is paginated; the browser initially renders 20 results.
- [ ] Add fixture-based startup, warm FTS, semantic, memory, and multilingual performance gates.
