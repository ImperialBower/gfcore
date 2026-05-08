


## Insight 1
Both auditors independently converged on the same 5 findings. The highest-risk issue is a classic "API over-promise": the trait defines 8 methods but the
engine only reads 6 of them. This is a form of Hyrum's Law in reverse — callers may reasonably depend on the advertised contract, silently getting wrong
behavior. The fix routes ask validation and book detection through the trait methods rather than hard-coding the standard logic.

## Insight 2
The is_book test requires careful deck design: the startup collect_books_for_player call (which will also go through rules.is_book() after the fix) would
immediately drain A's hand if A started with a pair. So the test deck interleaves ranks — ACE_SPADES, ACE_HEARTS, KING_SPADES, KING_HEARTS — so round-robin
dealing gives each player one ace and one king, no startup books, and the test proceeds cleanly.