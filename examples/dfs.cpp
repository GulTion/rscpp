// Connected components via DFS (LeetCode-style class methods).
// Graph: 0—1—2   3—4   5  → 3 components
// Run: cargo run -p rscpp-pipeline -- examples/dfs.cpp

class Solution {
public:
    void dfs(int u, vector<vector<int>>& adj, vector<int>& vis) {
        vis[u] = 1;
        int i = 0;
        while (i < adj[u].size()) {
            int v = adj[u][i];
            if (vis[v] == 0) {
                dfs(v, adj, vis);
            }
            i = i + 1;
        }
    }

    int countComponents(int n, vector<vector<int>>& adj) {
        vector<int> vis;
        int i = 0;
        while (i < n) {
            vis.push_back(0);
            i = i + 1;
        }
        int comps = 0;
        i = 0;
        while (i < n) {
            if (vis[i] == 0) {
                dfs(i, adj, vis);
                comps = comps + 1;
            }
            i = i + 1;
        }
        return comps;
    }
};

int main() {
    int n = 6;
    vector<vector<int>> adj = {{}, {}, {}, {}, {}, {}};
    adj[0].push_back(1);
    adj[1].push_back(0);
    adj[1].push_back(2);
    adj[2].push_back(1);
    adj[3].push_back(4);
    adj[4].push_back(3);
    Solution s;
    return s.countComponents(n, adj);
}
