# Known gaps (LeetCode C++ subset)

## Supported (common)

- `min` / `max` / `abs` (and `std::min`, `std::max`, `std::abs`, `labs`, `llabs`)
- `INT_MAX`, `INT_MIN`, `LONG_MAX`, `LONG_MIN`, `LLONG_MAX`, `LLONG_MIN`, `UINT_MAX`
- `swap`, `sort(v.begin(), v.end())`
- STL containers + methods (see `docs/events.md`)

## Not yet (frequently used on LeetCode)

- `minmax_element` (iterator pair return)
- `numeric_limits<T>::max()`

File bugs against the ones you hit first.
