# tunapond

A work in progress mining pool for Fortuna.

## How to Run
- Copy `.env.example` to `.env`
- Set your environment variables as defined in `.env`.
- `cargo install sqlx-cli`
- `sqlx database setup`
- `deno run --allow-all submission_server.ts`
- `cargo run --release`


## Kupo Matchers for preview

```
25637629.87f143f0565015a923da8f49d9c504835528c68d9a476dd0200696271e8713ac

502fbfbdafc7ddada9c335bd1440781e5445d08bada77dc2032866a6.54554e41	# Tuna Policy
addr_test1wpgzl0aa4lramtdfcv6m69zq0q09g3ws3wk6wlwzqv5xdfsdcf2qa	# Tuna Contract address
addr_testxxxxxxxx  # POOL MINING WALLET
addr_testxxxxxxxx  # THE POOL CONTRACT ADDRESS
```

## How generate a pool wallet
In `tunapond-client`

The `-p` is for preview.

```
deno run --allow-all main.ts new_wallet -p
```

You will need to fund this wallet with some (t)ADA.

## API

### Work
`GET /work?address={}`
Query for work to do. Importantly, this provides an _assigned nonce_. At minimum, miners MUST use the final 4 bytes of this provided nonce, as they uniquely identify the user and the pool.

Returns

```
type Block = {
    block_number: number
    current_hash: string
    leading_zeroes: number
    difficulty_number: number
    epoch_time: number
    current_time: number
    extra: string
    interlink: string[]
}

type Work = {
    nonce: string
	min_zeroes: number (u8)
    current_block: Block
	miner_id: number
}
```

### Submit
`POST /submit`

Submit a sample of SHA hashes to the server. Currently the expectation is that all hashes with `work.min_zeroes` and above are sent. There is no client distinction between "sampled" hashes that are used for measuring hashrate and true new blocks being found. This distinction is handled on the server. Any number of entries, including 0, is acceptable.

#### Request
```json
{
	"address": "addr1q9g0grcjlq0jeunpt27gy8w698zukar7wgsy9qp6fjkr23pcsufznxyrxw7j84uaypjvdk7yz3ft007hyx9wm6x7djssrusn86",
	"entries": [{
		"nonce": "6a6fe84d2ffb532fc097e4ad0173ef2e"
	}, {
		"nonce": "6a6fe84d2ffb532fc097e4ad0173ef2e"
	}]
}

```

#### Response
Contains information about the number of accepted hashes and the current chain head.

If `num_accepted` is lower than the number of hashes sent, consider reviewing the output locally for duplicate or invalid sha hashes.

Clients are expected to be mining the latest block by any means. While the server pool will provide an up-to-date view of the current block within the response of each submission, this still may lead to the rejection of some hashes due to them being calculated for an "old" block. 

```json
{
	"num_accepted": 2,
	"nonce": "249a83749bc3749df32",
	"working_block": {
		"block_number": 27523,
		"current_hash": "0000000000021f0f792e7694c6d7ccbb07e062727ec6850765094c61927f5399",
		"leading_zeroes": 11,
		"difficulty_number": 16383,
		"epoch_time": 89179000,
		"current_time": 1694226404000,
		"extra": "416c4c204861496c2074556e41",
		"interlink": [
			"0000000000013ed9abdc3f6addd91bb07a73b989c68dabab48aa4be409883afa",
			"000000000000777b771189508ecefddc36a6ff8cff7a2a35cbc2556467d9ae10"
		]
	}
}
```

### Hashrate
`GET /hashrate`

#### Request
`/hashrate?miner_id={}&start_time={}&end_time={}

Returns the estimated hashrate for the specified time period. Times are in UTC seconds.