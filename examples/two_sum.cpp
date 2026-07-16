// LeetCode 1. Two Sum — class Solution style (rscpp subset, no #include).
// Run: cargo run -p rscpp-pipeline -- examples/two_sum.cpp
// Expect: main() → Int(1)  meaning indices [0, 1] encoded as 0*10+1

class Solution {
public:
    vector<int> twoSum(vector<int>& nums, int target) {
        map<int, int> seen;
        for (int i = 0; i < nums.size(); ++i) {
            int need = target - nums[i];
            if (seen.count(need)) {
                return {seen[need], i};
            }
            seen[nums[i]] = i;
        }
        return {};
    }
};

int main() {
    vector<int> nums;
    nums.push_back(2);
    nums.push_back(7);
    nums.push_back(11);
    nums.push_back(15);

    Solution s;
    vector<int> ans = s.twoSum(nums, 9);
    // 2 + 7 == 9 → indices 0 and 1
    return ans[0] * 10 + ans[1];
}
