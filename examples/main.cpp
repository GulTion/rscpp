// Weighted adjacency matrix (rscpp subset — no #include / cout).
// Same idea as algo_vis/main.cpp: 3 nodes, edges 0→1=5, 1→2=7, 0→2=10.
// Run: cargo run -p rscpp-pipeline -- examples/main.cpp
// Returns sum of all weights (= 22).

int main() {
    vector<vector<int>> adj = {{0, 0, 0}, {0, 0, 0}, {0, 0, 0}};

    adj[0][1] = 5;
    adj[1][2] = 7;
    adj[0][2] = 10;

    int sum = 0;
    int i = 0;
    while (i < 3) {
        int j = 0;
        while (j < 3) {
            sum = sum + adj[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    return sum;
}
