// LeetCode 51. N-Queens — backtracking DFS (rscpp subset, no #include).
// Run: cargo run -p rscpp-wasm --example dump_complex_fixtures
// Expect: main() → Int(2)  (number of distinct solutions for n = 4)

class Solution {
public:
    void addAnswer(
        int n,
        vector<int>& chess,
        vector<vector<string>>& ans
    ) {
        vector<string> v;
        int i = 0;
        while (i < n) {
            string s;
            int j = 0;
            while (j < n) {
                if (j == chess[i]) {
                    s.push_back('Q');
                } else {
                    s.push_back('.');
                }
                j = j + 1;
            }
            v.push_back(s);
            i = i + 1;
        }
        ans.push_back(v);
    }

    void dfs(
        int row,
        int n,
        vector<int>& cols,
        vector<int>& main_diag,
        vector<int>& anti_diag,
        vector<int>& chess,
        vector<vector<string>>& ans
    ) {
        if (row == n) {
            addAnswer(n, chess, ans);
            return;
        }

        int i = 0;
        while (i < n) {
            if (cols[i] == 0 && main_diag[row + i] == 0 && anti_diag[row - i + n] == 0) {
                cols[i] = 1;
                main_diag[row + i] = 1;
                anti_diag[row - i + n] = 1;
                chess[row] = i;
                dfs(row + 1, n, cols, main_diag, anti_diag, chess, ans);
                cols[i] = 0;
                main_diag[row + i] = 0;
                anti_diag[row - i + n] = 0;
            }
            i = i + 1;
        }
    }

    vector<vector<string>> solveNQueens(int n) {
        vector<int> cols(n, 0);
        vector<int> main_diag(2 * n, 0);
        vector<int> anti_diag(2 * n, 0);
        vector<int> chess(n, 0);
        vector<vector<string>> ans;

        dfs(0, n, cols, main_diag, anti_diag, chess, ans);

        return ans;
    }
};

int main() {
    Solution s;
    vector<vector<string>> boards = s.solveNQueens(4);
    // n = 4 → 2 solutions (prefer run_method fixture; see dump_complex_fixtures)
    return boards.size();
}
