# chainbridge-substrate

[![Build Status](https://travis-ci.com/ChainSafe/chainbridge-substrate.svg?branch=master)](https://travis-ci.com/ChainSafe/chainbridge-substrate)

Substrate implementation for [ChainBridge](https://github.com/ChainSafe/ChainBridge). 

This repo contains two pallets:

## chainbridge

The core bridge logic. This handles voting and execution of proposals, administration of the relayer set and signaling transfers.


## example-pallet

This pallet demonstrates how the chainbridge pallet can be integrated in to a substrate chain. It implements calls that can be executed through proposal only and to initiate a basic transfer across the bridge.

## example-erc721

This pallet mimics an ERC721 token contract. It allows for minting, burning and transferring of tokens that consist of a token ID (`U256`) and some metadata (`Vec<u8>`). This is also integrated into `example-pallet` to demonstrate how non-fungibles can be transferred across the bridge.
