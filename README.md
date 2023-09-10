# tunapond

A work in progress mining pool for Fortuna.

## How to Run
- Copy `.env.example` to `.env`
- Set your `KUPO_URL` in `.env`.
- `cargo install sqlx-cli`
- `sqlx database setup`
- `deno run --allow-all submission_server.ts`
- `cargo run --release`

## API

### Work
`GET /work`

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
    current_block: Block
}
```

### Submit
`POST /submit`

Submit a sample of SHA hashes to the server. Currently the expectation is that all hashes with 8 zeroes and above are sent, but this is not yet enforced. There is no client distinction between "sampled" hashes that are used for measuring hashrate and true new blocks being found. This distinction is handled on the server. Any number of entries, including 0, is acceptable.

#### Request
```json
{
	"address": "addr1q9g0grcjlq0jeunpt27gy8w698zukar7wgsy9qp6fjkr23pcsufznxyrxw7j84uaypjvdk7yz3ft007hyx9wm6x7djssrusn86",
	"entries": [{
		"sha": "0000000000035fd225f35b60e69db5f3184c88419026b93560d951edb2636a11",
		"nonce": "6a6fe84d2ffb532fc097e4ad0173ef2e"
	}, {
		"sha": "0000000000035fd225f35b60e69db5f3184c88419026b93560d951edbfff6b3",
		"nonce": "6a6fe84d2ffb532fc097e4ad0173ef2e"
	}]
}

```

#### Response
Contains information about the number of accepted hashes and the current chain head.

If `num_accepted` is lower than the number of hashes sent, consider reviewing the output locally for duplicate or invalid sha hashes.

If `block_number` has changed since the last time the client has submitted or registered, the client MUST begin mining the new block. Failure to do so will result in rejection of all new submissions.

```json
{
	"num_accepted": 2,
	"session_id": 43,
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
`/hashrate?address={}

TODO!