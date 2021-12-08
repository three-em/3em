// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.10;

contract Name {
  string name;
  
  constructor() public {
    name = "littledivy";
  }

  function set_name( string memory new_name) public {
    name = new_name;
  }

  function get_name()  public view returns(string memory) {
    return name;
  }
}
