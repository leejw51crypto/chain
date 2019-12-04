#!/usr/bin/env python3
import sys
import base64
import hashlib
import json
import asyncio
import tempfile
from pathlib import Path
import re
import os
import configparser
import binascii

import jsonpatch
import fire
import toml
import nacl.signing
from nacl.encoding import HexEncoder
from decouple import config

ROOT_PATH = Path(config('ROOT_PATH', '.')).resolve()
BASE_PORT = config('BASE_PORT', 26650, cast=int)
SGX_DEVICE = config('SGX_DEVICE', None)
SGX_MODE = 'HW' if SGX_DEVICE else 'SW'

CHAIN_TX_ENCLAVE_DOCKER_IMAGE = config('CHAIN_TX_ENCLAVE_DOCKER_IMAGE',
                                       'integration-tests-chain-tx-enclave')
CHAIN_ID = config('CHAIN_ID', 'test-ab')

DEVUTIL_CMD = Path('dev-utils')
CLIENT_CMD = Path('client-cli')
CHAIN_CMD = Path('chain-abci')
CLIENT_RPC_CMD = Path('client-rpc')


class SigningKey:
    def __init__(self, seed):
        self._seed = seed
        self._sk = nacl.signing.SigningKey(seed, HexEncoder)

    def priv_key_base64(self):
        return base64.b64encode(self._sk._signing_key).decode()

    def pub_key_base64(self):
        vk = self._sk.verify_key
        return base64.b64encode(bytes(vk)).decode()

    def validator_address(self):
        vk = self._sk.verify_key
        return hashlib.sha256(bytes(vk)).hexdigest()[:40].upper()


def tendermint_cfg(moniker, app_port, rpc_port, p2p_port, peers):
    return {
        'proxy_app': 'tcp://127.0.0.1:%d' % app_port,
        'moniker': moniker,
        'fast_sync': True,
        'db_backend': 'goleveldb',
        'db_dir': 'data',
        'log_level': 'main:info,state:info,*:error',
        #'log_level': '*:debug',
        'log_format': 'plain',
        'genesis_file': 'config/genesis.json',
        'priv_validator_key_file': 'config/priv_validator_key.json',
        'priv_validator_state_file': 'data/priv_validator_state.json',
        'priv_validator_laddr': '',
        'node_key_file': 'config/node_key.json',
        'abci': 'socket',
        'prof_laddr': '',
        'filter_peers': False,
        'rpc': {
            'laddr': 'tcp://127.0.0.1:%d' % rpc_port,
            'cors_allowed_origins': [],
            'cors_allowed_methods': [
                'HEAD',
                'GET',
                'POST'
            ],
            'cors_allowed_headers': [
                'Origin',
                'Accept',
                'Content-Type',
                'X-Requested-With',
                'X-Server-Time'
            ],
            'grpc_laddr': '',
            'grpc_max_open_connections': 900,
            'unsafe': False,
            'max_open_connections': 900,
            'max_subscription_clients': 100,
            'max_subscriptions_per_client': 5,
            'timeout_broadcast_tx_commit': '10s',
            'max_body_bytes': 1000000,
            'max_header_bytes': 1048576,
            'tls_cert_file': '',
            'tls_key_file': ''
        },
        'p2p': {
            'laddr': 'tcp://0.0.0.0:%d' % p2p_port,
            'external_address': '',
            'seeds': '',
            'persistent_peers': peers,
            'upnp': False,
            'addr_book_file': 'config/addrbook.json',
            'addr_book_strict': False,
            'max_num_inbound_peers': 40,
            'max_num_outbound_peers': 10,
            'flush_throttle_timeout': '100ms',
            'max_packet_msg_payload_size': 1024,
            'send_rate': 5120000,
            'recv_rate': 5120000,
            'pex': True,
            'seed_mode': False,
            'private_peer_ids': '',
            'allow_duplicate_ip': True,
            'handshake_timeout': '20s',
            'dial_timeout': '3s'
        },
        'mempool': {
            'recheck': True,
            'broadcast': True,
            'wal_dir': '',
            'size': 5000,
            'max_txs_bytes': 1073741824,
            'cache_size': 10000,
            'max_tx_bytes': 1048576
        },
        'fastsync': {'version': 'v0'},
        'consensus': {
            'wal_file': 'data/cs.wal/wal',
            'timeout_propose': '3s',
            'timeout_propose_delta': '500ms',
            'timeout_prevote': '1s',
            'timeout_prevote_delta': '500ms',
            'timeout_precommit': '1s',
            'timeout_precommit_delta': '500ms',
            'timeout_commit': '1s',
            'skip_timeout_commit': False,
            'create_empty_blocks': True,
            'create_empty_blocks_interval': '0s',
            'peer_gossip_sleep_duration': '100ms',
            'peer_query_maj23_sleep_duration': '2s'
        },
        'tx_index': {
            'indexer': 'kv',
            'index_tags': '',
            'index_all_tags': True
        },
        'instrumentation': {
            'prometheus': False,
            'prometheus_listen_addr': ':26660',
            'max_open_connections': 3,
            'namespace': 'tendermint'
        }
    }


def priv_validator_key(seed):
    sk = SigningKey(seed)
    return {
        'address': sk.validator_address(),
        'pub_key': {
            'type': 'tendermint/PubKeyEd25519',
            'value': sk.pub_key_base64(),
        },
        'priv_key': {
            'type': 'tendermint/PrivKeyEd25519',
            'value': sk.priv_key_base64(),
        },
    }


def node_key(seed):
    sk = SigningKey(seed)
    return {
        'priv_key':{
            'type':'tendermint/PrivKeyEd25519',
            'value': sk.priv_key_base64(),
        }
    }


def app_state_cfg(cfg):
    return {
        "rewards_pool": str(cfg['rewards_pool']),
        "distribution": gen_distribution(cfg['nodes']),
        "unbonding_period": 60,
        "required_council_node_stake": "1",
        "jailing_config": {
            "jail_duration": 86,
            "block_signing_window": 20,
            "missed_block_threshold": 10
        },
        "slashing_config": {
            "liveness_slash_percent": "0.1",
            "byzantine_slash_percent": "0.2",
            "slash_wait_period": 10
        },
        "initial_fee_policy": {
            "base_fee": "1.1",
            "per_byte_fee": "1.25"
        },
        "council_nodes": {
            node['staking'][0]: [
                node['name'],
                '%s@example.com' % node['name'],
                {
                    'consensus_pubkey_type': 'Ed25519',
                    'consensus_pubkey_b64': SigningKey(node['validator_seed']).pub_key_base64(),
                }
            ]
            for node in cfg['nodes']
        },
        "genesis_time": cfg['genesis_time'],
    }


def programs(node, app_hash):
    node_path = ROOT_PATH / Path(node['name'])
    base_port = node['base_port']
    chain_abci_port = base_port + 8
    tendermint_rpc_port = base_port + 7
    client_rpc_port = base_port + 1
    commands = [
        ('tx-enclave', f'''docker run --rm -p {base_port}:25933 --env RUST_BACKTRACE=1 --env RUST_LOG=info -v {node_path / Path('enclave')}:/enclave-storage {'--device ' + SGX_DEVICE if SGX_DEVICE else ''} {CHAIN_TX_ENCLAVE_DOCKER_IMAGE}-{SGX_MODE.lower()}'''),
        ('chain-abci', f'''{CHAIN_CMD} -g {app_hash} -c {CHAIN_ID} --enclave_server tcp://127.0.0.1:{base_port} --data {node_path / Path('chain')} -p {chain_abci_port}'''),
        ('tendermint', f'''tendermint node --home={node_path / Path('tendermint')}'''),
        ('client-rpc', f'''{CLIENT_RPC_CMD} --port={client_rpc_port} --chain-id={CHAIN_ID} --storage-dir={node_path / Path('wallet')} --websocket-url=ws://127.0.0.1:{tendermint_rpc_port}/websocket'''),
    ]

    return {
        'program:%s-%s' % (name, node['name']): {
            'command': cmd,
            'stdout_logfile': f"%(here)s/{name}-%(group_name)s.log",
            'environment': 'RUST_BACKTRACE=1,RUST_LOG=info',
            'autostart': 'true',
            'autorestart': 'true',
            'redirect_stderr': 'true',
            'priority': str(priority),
            'startsecs': '1',
            'startretries': '10',
        }
        for priority, (name, cmd) in enumerate(commands)
    }


def tasks_ini(node_cfgs, app_hash):
    ini = {
        'supervisord': {
            'pidfile': '%(here)s/supervisord.pid',
        },
        'rpcinterface:supervisor': {
            'supervisor.rpcinterface_factory': 'supervisor.rpcinterface:make_main_rpcinterface',
        },
        'unix_http_server': {
            'file': '%(here)s/supervisor.sock',
        },
        'supervisorctl': {
            'serverurl': 'unix://%(here)s/supervisor.sock',
        },
    }

    for node in node_cfgs:
        prgs = programs(node, app_hash)
        ini['group:%s' % node['name']] = {
            'programs': ','.join(name.split(':', 1)[1]
                                 for name in prgs.keys()),
        }
        ini.update(prgs)

    return ini


def write_tasks_ini(fp, cfg):
    ini = configparser.ConfigParser()
    for section, items in cfg.items():
        ini.add_section(section)
        sec = ini[section]
        sec.update(items)
    ini.write(fp)


def coin_to_voting_power(coin):
    return int(int(coin) / (10 ** 8))


async def run(cmd, ignore_error=False, **kwargs):
    proc = await asyncio.create_subprocess_shell(cmd, **kwargs)
    retcode = await proc.wait()
    if not ignore_error:
        assert retcode == 0, cmd


async def interact(cmd, input=None, **kwargs):
    proc = await asyncio.create_subprocess_shell(
        cmd,
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        **kwargs
    )
    (stdout, stderr) = await proc.communicate(input=input)
    assert proc.returncode == 0, f'{stdout.decode("utf-8")} ({cmd})'
    return stdout


async def gen_app_state(cfg):
    with tempfile.NamedTemporaryFile('w') as fp:
        json.dump(cfg, fp)
        fp.flush()
        result = await interact(f'{DEVUTIL_CMD} genesis generate -g "{fp.name}"')
        print(result)
        return json.loads('{%s}' % result.decode('utf-8'))


async def gen_wallet_addr(mnemonic, type='Staking', count=1):
    prefix = {
        'Staking': '0x',
        'Transfer': 'dcro',
    }[type]
    with tempfile.TemporaryDirectory() as dirname:
        await interact(
            f'{CLIENT_CMD} wallet restore --name Default',
            ('123456\n123456\n%s\n%s\n' % (mnemonic, mnemonic)).encode(),
            env=dict(
                os.environ,
                CRYPTO_CLIENT_STORAGE=dirname,
            ),
        )
        addrs = []
        for i in range(count):
            result = (await interact(
                f'{CLIENT_CMD} address new --name Default --type {type}',
                b'123456\n',
                env=dict(
                    os.environ,
                    CRYPTO_CLIENT_STORAGE=dirname,
                ),
            )).decode()
            addrs.append(re.search(prefix + r'[0-9a-zA-Z]+', result).group())
        return addrs


async def gen_genesis(cfg):
    genesis = {
        "genesis_time": cfg['genesis_time'],
        "chain_id": CHAIN_ID,
        "consensus_params": {
            "block": {
                "max_bytes": "22020096",
                "max_gas": "-1",
                "time_iota_ms": "1000"
            },
            "evidence": {
                "max_age": "100000"
            },
            "validator": {
                "pub_key_types": [
                    "ed25519"
                ]
            }
        },
        'validators': [
            {
                'address': SigningKey(node['validator_seed']).validator_address(),
                'pub_key': {
                    'type': 'tendermint/PubKeyEd25519',
                    'value': SigningKey(node['validator_seed']).pub_key_base64(),
                },
                'power': str(coin_to_voting_power(node['bonded_coin'])),
                'name': node['name'],
            }
            for node in cfg['nodes']
        ],
    }

    patch = jsonpatch.JsonPatch(cfg['config_patch'])
    state = await gen_app_state(patch.apply(app_state_cfg(cfg)))
    genesis.update(state)
    return genesis


def gen_validators(cfgs):
    return [
        (
            cfg['staking'][0],
            SigningKey(cfg['validator_seed']),
            coin_to_voting_power(cfg['bonded_coin']),
            cfg['name'],
        )
        for cfg in cfgs
    ]


def gen_distribution(nodes):
    dist = {
        node['staking'][0]: str(node['bonded_coin'])
        for node in nodes
    }
    for node in nodes:
        dist[node['staking'][1]] = str(node['unbonded_coin'])
    return dist


def gen_peers(cfgs):
    return ','.join(
        'tcp://%s@0.0.0.0:%d' % (
            SigningKey(cfg['node_seed']).validator_address().lower(),
            cfg['base_port'] + 6
        )
        for i, cfg in enumerate(cfgs)
    )


async def init_cluster(cfg):
    await populate_wallet_addresses(cfg['nodes'])

    peers = gen_peers(cfg['nodes'])
    genesis = await gen_genesis(cfg)
    app_hash = genesis['app_hash']

    node_cfg= cfg['nodes'][0]
    info= {"app_hash":app_hash, "seed_id":  SigningKey(node_cfg['node_seed']).validator_address().lower()}
    # write
    json.dump(info,
                open(ROOT_PATH / Path('info.json'), 'w'),
                indent=4)
    # write nodes
    json.dump(cfg,
                open(ROOT_PATH / Path('nodes_info.json'), 'w'),
                indent=4)

    # write
    f = open("run_test_env.sh", "w")
    f.write("export APP_HASH={}\n".format(info["app_hash"]))
    f.write("export SEED_ID={}\n".format(info["seed_id"]))
    f.close()
    

    for i, node in enumerate(cfg['nodes']):
        node_name = 'node%d' % i
        cfg_path = ROOT_PATH / Path(node_name) / Path('tendermint') / Path('config')
        if not cfg_path.exists():
            os.makedirs(cfg_path)

        json.dump(genesis,
                  open(cfg_path / Path('genesis.json'), 'w'),
                  indent=4)
        json.dump(node_key(node['node_seed']),
                  open(cfg_path / Path('node_key.json'), 'w'),
                  indent=4)
        json.dump(node_key(node['validator_seed']),
                  open(cfg_path / Path('priv_validator_key.json'), 'w'),
                  indent=4)
        toml.dump(tendermint_cfg(node_name,
                                 26658,
                                 26657,
                                 26656,
                                 ""),
                  open(cfg_path / Path('config.toml'), 'w'))

        data_path = ROOT_PATH / Path(node_name) / Path('tendermint') / Path('data')
        if not data_path.exists():
            data_path.mkdir()
        json.dump({
            "height": "0",
            "round": "0",
            "step": 0
        }, open(data_path / Path('priv_validator_state.json'), 'w'))

    write_tasks_ini(open(ROOT_PATH / Path('tasks.ini'), 'w'),
                    tasks_ini(cfg['nodes'], app_hash))


def gen_mnemonic():
    import mnemonic
    return mnemonic.Mnemonic('english').generate(160)


def gen_seed():
    return binascii.hexlify(os.urandom(32)).decode()


async def populate_wallet_addresses(nodes):
    for node in nodes:
        node['staking'] = await gen_wallet_addr(node['mnemonic'], type='Staking', count=2)
        # node['transfer'] = await gen_wallet_addr(node['mnemonic'], type='Transfer', count=3)


class CLI:
    def gen(self, count=1, rewards_pool=0,
            genesis_time="2019-11-20T08:56:48.618137Z",
            base_fee='0.0', per_byte_fee='0.0'):
        '''Generate testnet node specification
        :param count: Number of nodes, [default: 1].
        '''
        max_coin = 10000000000000000000
        share = int(int(max_coin - rewards_pool) / count / 2)
        cfg = {
            'genesis_time': genesis_time,
            'rewards_pool': rewards_pool,
            'nodes': [
                {
                    'name': 'node%d' % i,
                    'mnemonic': gen_mnemonic(),
                    'validator_seed': gen_seed(),
                    'node_seed': gen_seed(),
                    'bonded_coin': share,
                    'unbonded_coin': share,
                    'base_port': BASE_PORT + (i * 10),
                }
                for i in range(count)
            ],
            'config_patch': [
                {'op': 'replace', 'path': '/initial_fee_policy/base_fee', 'value': '0.0'},
                {'op': 'replace', 'path': '/initial_fee_policy/per_byte_fee', 'value': '0.0'},
            ],
        }
        print(json.dumps(cfg, indent=4))
        return cfg

    def prepare(self, spec=None):
        '''Prepare tendermint testnet based on specification
        :param spec: Path of specification file, [default: stdin]
        '''
        cfg = json.load(open(spec) if spec else sys.stdin)
        asyncio.run(init_cluster(cfg))
        print('Prepared succesfully', ROOT_PATH)

    def prepare_cfg(self, cfg, spec=None):
        '''Prepare tendermint testnet based on specification
        :param spec: Path of specification file, [default: stdin]
        '''
        asyncio.run(init_cluster(cfg))
        print('Prepared succesfully', ROOT_PATH)


if __name__ == '__main__':
    fire.Fire(CLI())
