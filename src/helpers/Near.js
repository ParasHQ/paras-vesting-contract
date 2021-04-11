const { Contract, KeyPair, connect } = require('near-api-js')
const { join } = require('path')
const { InMemoryKeyStore } = require('near-api-js').keyStores
const getConfig = require('../configs/near')

const Base64 = require('js-base64').Base64
const nacl = require('tweetnacl')
const bs58 = require('bs58')
const sha256 = require('js-sha256')
const axios = require('axios')

const _hexToArr = (str) => {
	try {
		return new Uint8Array(
			str.match(/.{1,2}/g).map((byte) => parseInt(byte, 16))
		)
	} catch (err) {
		throw err
	}
}

const contractConfig = {
	changeMethods: [
		'new',
        'claim_vested',
		'revoke',
	],
    viewMethods: [
		'recipient',
		'amount',
		'token',
		'amount_claimed',
		'cliff',
		'start',
		'duration',
		'revocable',
		'calculate_amount_vested',
        'releasable_amount',
    ]
}

class Near {
	constructor() {
		this.ctx = null
		this.config = null
	}

	async init() {
		console.log('==================================================')
		console.log(`ENV: ${process.env.NODE_ENV}`)
		const ROOT_ACCOUNT =
			process.env[`${process.env.NODE_ENV.toUpperCase()}_ROOT_ACCOUNT`]
		const CONTRACT_ACCOUNT =
			process.env[`${process.env.NODE_ENV.toUpperCase()}_CONTRACT_ACCOUNT`]

		if (!ROOT_ACCOUNT) {
			throw '[env] ROOT_ACCOUNT not found'
		}
		if (!CONTRACT_ACCOUNT) {
			throw '[env] CONTRACT_ACCOUNT not found'
		}
		const rootAccount = JSON.parse(ROOT_ACCOUNT)
		const contractAccount = JSON.parse(CONTRACT_ACCOUNT)
		console.log(`ROOT ACCOUNT: ${rootAccount.account_id}`)
		console.log(`CONTRACT ACCOUNT: ${contractAccount.account_id}`)
		console.log('==================================================')
		const config = getConfig(
			process.env.NODE_ENV || 'testnet',
			contractAccount.account_id
		)
		this.config = config

		const keyStore = new InMemoryKeyStore()

		// add root account
		const rootKeyPair = KeyPair.fromString(
			rootAccount.secret_key || rootAccount.private_key
		)
		await keyStore.setKey(config.networkId, rootAccount.account_id, rootKeyPair)

		// add contract account
		const contractKeyPair = KeyPair.fromString(
			contractAccount.secret_key || contractAccount.private_key
		)
		await keyStore.setKey(
			config.networkId,
			contractAccount.account_id,
			contractKeyPair
		)

		const near = await connect({
			deps: {
				keyStore: keyStore,
			},
			...config,
		})
		this.ctx = near
		this.masterAccount = await near.account(rootAccount.account_id)
		this.contractAccount = await near.account(contractAccount.account_id)
		this.contract = new Contract(
			this.masterAccount,
			this.contractAccount.accountId,
			contractConfig
		)
	}

	async deployContract() {
		console.log('Setting up and deploying contract')
		const contractPath = join(process.cwd(), 'res/paras_vesting_contract.wasm')
		await this.contractAccount.deployContract(
			require('fs').readFileSync(contractPath)
		)

		console.log(`Contract ${this.contractAccount.accountId} deployed`)
	}

	async authSignature(authHeader) {
		try {
			const decodeAuthHeader = Base64.decode(authHeader)
			const [userId, pubKey, signature] = decodeAuthHeader.split('&')
			const pubKeyArr = _hexToArr(pubKey)
			const signatureArr = _hexToArr(signature)
			const hash = new Uint8Array(sha256.sha256.array(userId))
			const verify = nacl.sign.detached.verify(hash, signatureArr, pubKeyArr)
			if (!verify) {
				throw new Error('unauthorized')
			}
			const b58pubKey = bs58.encode(Buffer.from(pubKey.toUpperCase(), 'hex'))
			const response = await axios.post(this.config.nodeUrl, {
				jsonrpc: '2.0',
				id: 'dontcare',
				method: 'query',
				params: {
					request_type: 'view_access_key',
					finality: 'final',
					account_id: userId,
					public_key: `ed25519:${b58pubKey}`,
				},
			})

			if (response.data.result && response.data.result.error) {
				console.log(response.data.result.error)
				throw new Error('unauthorized')
			}
			return userId
		} catch (err) {
			return null
		}
	}
}

module.exports = Near
