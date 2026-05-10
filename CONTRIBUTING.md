# Contributing to PiDCTP

Thank you for your interest in contributing to the Pi Decentralized Commerce & Trust Protocol (PiDCTP)! We welcome contributions from all Pioneers and developers.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Pull Request Process](#pull-request-process)
- [Specification Changes](#specification-changes)
- [Smart Contract Contributions](#smart-contract-contributions)
- [Community Feedback](#community-feedback)

## Code of Conduct

This project adheres to the Pi Network community standards. We expect all contributors to be respectful, constructive, and collaborative.

## How to Contribute

### Reporting Issues

1. Check existing issues before opening a new one
2. Use the issue templates provided
3. Include:
   - Clear description of the issue
   - Steps to reproduce (if applicable)
   - Expected vs. actual behavior
   - Relevant section of the specification (e.g., "PiRC3 Section 3: Escrow")

### Suggesting Enhancements

1. Open a GitHub Discussion first to gauge community interest
2. Reference the specific PiRC3 section your enhancement relates to
3. Explain why the enhancement is needed and how it aligns with PiDCTP's vision
4. Provide examples of how the enhancement would work in practice

## Development Setup

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add Soroban target
rustup target add wasm32-unknown-unknown

# Install Soroban CLI
cargo install soroban-cli --version 20.0.0

# Clone the repository
git clone https://github.com/aybvip/PiRC.git
cd PiRC
```

### Build

```bash
# Build all contracts
soroban contract build

# Run tests
cargo test
```

### Project Structure

```
PiRC/
├── PiRC3/              # Specification documents
├── contracts/          # Soroban smart contracts
│   ├── escrow/         # Escrow module
│   ├── reputation/     # Reputation module
│   ├── dispute/        # Dispute resolution module
│   ├── merchant/       # Merchant verification module
│   ├── loyalty/        # Loyalty & rewards module
│   ├── coordinator/    # Entry point & router
│   └── shared/         # Shared types
└── tests/              # Integration tests
```

## Pull Request Process

1. **Fork** the repository
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** following the coding standards below
4. **Test your changes**:
   ```bash
   cargo test
   soroban contract build
   ```
5. **Commit with clear messages**:
   ```bash
   git commit -m "feat(escrow): add auto-release incentive mechanism"
   ```
6. **Push and open a Pull Request**

### Commit Message Format

```
type(scope): description

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `spec`: Specification changes
- `test`: Adding tests
- `refactor`: Code refactoring
- `security`: Security-related changes

**Scopes:**
- `escrow`, `reputation`, `dispute`, `merchant`, `loyalty`, `coordinator`, `shared`, `spec`

### PR Review Criteria

- [ ] All tests pass
- [ ] Code follows existing style and patterns
- [ ] Changes are documented in the relevant specification section
- [ ] No breaking changes without prior discussion
- [ ] Security implications considered

## Specification Changes

Changes to the PiRC3 specification (documents in `PiRC3/`) follow a more formal process:

1. **Open a Discussion** proposing the change with rationale
2. **Gather community feedback** (minimum 7 days)
3. **Draft the change** as a PR with updated specification
4. **Review period** (minimum 14 days for substantive changes)
5. **Merge** after consensus is reached

### Specification Versioning

- Minor clarifications: Direct PR
- New sections or modules: Discussion + PR
- Changes to core design (Sections 1-2): Full community review process
- Error code or data type changes: PR with cross-reference impact analysis

## Smart Contract Contributions

### Coding Standards

- Use `#![no_std]` in all contracts
- Follow Checks-Effects-Interactions pattern
- All state changes before external calls
- Use Soroban's checked arithmetic (default)
- Emit events on all state-changing operations
- Validate all inputs at the contract boundary

### Testing Requirements

- Unit tests for every public function
- Integration tests for cross-module interactions
- Edge case tests (zero amounts, maximum values, expired deadlines)
- Error path tests (unauthorized access, invalid state transitions)

### Security Checklist

Before submitting a PR for a smart contract:

- [ ] No uninitialized storage access
- [ ] All state transitions validated
- [ ] Access control enforced
- [ ] No reentrancy vectors
- [ ] Events emitted for all state changes
- [ ] Fee calculations correct for edge cases
- [ ] Overflow/underflow handled (Soroban default)

## Community Feedback

For Pioneers without a GitHub account:
- Submit high-level feedback through the Pi Network community channels
- Engage in Discussions on this repository

For developers:
- Open Issues for bugs and suggestions
- Submit Pull Requests for code changes
- Participate in code reviews

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
