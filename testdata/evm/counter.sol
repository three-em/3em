pragma solidity ^0.8.10;

contract TestCounter {
    constructor() public {
        count = 0;
        count2 = 100;
    }

    function incrementCounter() public {
        count += 10;
        count2 += 2;
    }

    function decrementCounter() public {
        count -= 1;
    }

    function getCount() public view returns (int) {
        return count;
    }

    function getCount2() public view returns (int) {
        return count2;
    }
}