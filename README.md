# SOON Server Staking Contract

## Overview
A decentralized staking contract built on Solana that enables server operators to participate in network validation through both direct staking and delegation mechanisms. This contract implements a flexible staking system with advanced features for server management and delegation.

## Key Features
- **Server Registration**: Operators can register servers with unique identifiers and custom names
- **Flexible Staking**: Support for both direct staking and delegation mechanisms
- **Stake Limits**: Built-in minimum and maximum stake limits to ensure network stability
- **Delegation System**: Users can delegate tokens to registered servers
- **Account Management**: Comprehensive account system for tracking stakes and delegations
- **Safety Features**: Built-in guards against common attack vectors and error conditions

## Technical Specifications
- **Token Standard**: SPL Token compatible
- **Minimum Server Stake**: 1,000 tokens
- **Maximum Server Stake**: 10,000 tokens
- **Minimum Delegation**: 500 tokens
- **Contract Version**: 1.0

## Core Functionalities
- Server registration and management
- Direct token staking
- Delegation support
- Stake/unstake operations
- Comprehensive event emission
- Automated account creation

## Security Features
- Ownership verification
- Balance checks
- Initialization guards
- Mint address verification
- Non-zero balance protection
- Overflow protection

## Account Structure
- Main contract account for global state
- Server info accounts (PDA)
- Delegation accounts
- Token vaults

## Events
The contract emits events for all major operations:
- Server registration/updates/removal
- Stake deposits/withdrawals
- Delegation operations
- Account management

## Requirements
- Solana Program Library (SPL)
- Anchor Framework
- Associated Token Program

## Usage
The contract supports various staking operations through its instruction set:
```rust
- initialize_main()     // Initialize the main contract
- add_server()         // Register a new server
- update_server()      // Update server information
- deposit()           // Stake tokens
- withdraw()          // Withdraw staked tokens
- d_deposit()         // Delegate tokens
- d_withdraw()        // Withdraw delegated tokens
```

## Installation
[Installation instructions to be added]

## Security Considerations
- All operations include ownership verification
- Built-in protection against double initialization
- Balance checks before operations
- Safe math operations to prevent overflows
- Proper PDA (Program Derived Address) usage

## Error Handling
Comprehensive error handling for common scenarios:
- Insufficient funds
- Invalid mint addresses
- Unauthorized operations
- Account initialization conflicts
- Stake limit violations

## License
[License information to be added]
