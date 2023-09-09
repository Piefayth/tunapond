# tunapond

A work in progress mining pool for Fortuna.

## How to Run
- Copy `.env.example` to `.env`
- Set your `KUPO_URL` in `.env`.
- `cargo run --release`

## API

### Register
`POST /register`

Start a new mining session.

Takes a wallet-signed message with an arbitrary payload. The payload being arbitrary is temporary - expect a breaking change.

#### Request
```json
{
	"address": "addr1q9g0grcjlq0jeunpt27gy8w698zukar7wgsy9qp6fjkr23pcsufznxyrxw7j84uaypjvdk7yz3ft007hyx9wm6x7djssrusn86",
	"payload": "eventually this message will matter and be validated",
	"key": "a40101032720062158206a52560327885ea742d6b7334d75d78d2b11775bfd1f08aabcb366adb448b3fb",
	"signature": "845846a20127676164647265737358390150f40f12f81f2cf2615abc821dda29c5cb747e722042803a4cac354438871229988333bd23d79d2064c6dbc41452b7bfd7218aede8de6ca1a166686173686564f4447465737458400ce79a0cbfe6a66d77af0f1e88372d9b7246f387360435b290605366bb74574138f5af9b4bf9bf36ad69a51bd2cd365e03a8589a96adad2d3e9cc9677e87e70c"
}
```

Example of constructing such a message with Lucid:

```ts
const address = await lucid.wallet.address()
const payload = "eventually this message will matter and be validated"
const payloadHex = fromText(payload)
const signed = await lucid.wallet.signMessage(address, payloadHex)
const registration: Registration = {
    address,
    payload,
    ...signed
}
```

#### Response

Contains information about the started session, including the current block to mine. Miners are expected to ONLY mine the block the pool has told them to, regardless of them having seen an earlier block through some other mechanism. This response is sufficient information to begin mining immediately.

```json
{
	"address": "addr1q9g0grcjlq0jeunpt27gy8w698zukar7wgsy9qp6fjkr23pcsufznxyrxw7j84uaypjvdk7yz3ft007hyx9wm6x7djssrusn86",
	"message": "Started a new mining session.",
	"session_id": 9,
	"start_time": "2023-09-08T23:50:35.906673300",
	"current_block": {
		"block_number": 27296,
		"current_hash": "000000000003e270c6c335bef41c21bd682dc83489c27724d37e24dd3eb09ef8",
		"leading_zeroes": 11,
		"difficulty_number": 16383,
		"epoch_time": 79826000,
		"current_time": 1694217051000,
		"extra": "416c4c204861496c2074556e41",
		"interlink": [
			"000000000000dd784f38e2addf70011151fdeef4465710908b6ce930665b7e9b",
			"000000000000dd784f38e2addf70011151fdeef4465710908b6ce930665b7e9b"
		]
	}
}
```

### Deregister
`POST /deregister`

End a mining session.

#### Request
Identical to `/register`.

#### Response
```json
{
	"message": "Ended mining session for addr1q9g0grcjlq0jeunpt27gy8w698zukar7wgsy9qp6fjkr23pcsufznxyrxw7j84uaypjvdk7yz3ft007hyx9wm6x7djssrusn86."
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
`/hashrate?session_id={}`

Gets the estimated hashrate for the specified session.

#### Response
```json
{
	"estimated_hashes_per_second": 4238454568.4210525
}
```