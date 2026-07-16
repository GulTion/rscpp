// LeetCode 20. Valid Parentheses — subset-friendly (no map iterators).
// Run: cargo run -p rscpp-pipeline -- examples/valid_parentheses.cpp
// Or: run_method(src, "Solution::isValid", ["()[]{}"])

class Solution {
public:
    bool isValid(string s) {
        stack<char> st;
        for (const auto& c : s) {
            if (c == '(' || c == '[' || c == '{') {
                st.emplace(c);
            } else {
                if (st.empty()) {
                    return false;
                }
                char top = st.top();
                st.pop();
                if (c == ')' && top != '(') {
                    return false;
                }
                if (c == ']' && top != '[') {
                    return false;
                }
                if (c == '}' && top != '{') {
                    return false;
                }
            }
        }
        return st.empty();
    }
};

int main() {
    Solution sol;
    // "()[]{}" is valid → return 1
    if (!sol.isValid("()[]{}")) {
        return 0;
    }
    // "(]" is invalid → return 1 only if both checks pass
    if (sol.isValid("(]")) {
        return 0;
    }
    return 1;
}
