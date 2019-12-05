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


def tendermint_cfg(moniker, hostname, app_port, rpc_port, p2p_port, peers):
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
            'laddr': 'tcp://%s:%d' % (hostname, rpc_port),
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
        'priv_key': {
            'type': 'tendermint/PrivKeyEd25519',
            'value': sk.priv_key_base64(),
        }
    }


def app_state_cfg(cfg):
    return {
        "distribution": gen_distribution(cfg),
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
        "rewards_config": {
            "monetary_expansion_cap": str(cfg['expansion_cap']),
            "distribution_period": 86400,
            "monetary_expansion_r0": 500,
            "monetary_expansion_tau": 145000000,
            "monetary_expansion_decay": 999860
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


def programs(node, app_hash, root_path, cfg):
    node_path = root_path / Path(node['name'])
    base_port = node['base_port']
    chain_abci_port = base_port + 8
    tendermint_rpc_port = base_port + 7
    client_rpc_port = base_port + 1
    sgx_device = cfg.get('sgx_device')
    commands = [
        ('tx-enclave', f'''docker run --rm -p {base_port}:25933 --env RUST_BACKTRACE=1 --env RUST_LOG=info -v {node_path / Path('enclave')}:/enclave-storage {'--device ' + sgx_device if sgx_device else ''} {cfg['enclave_docker_image']}'''),
        ('chain-abci', f'''chain-abci -g {app_hash} -c {cfg['chain_id']} --enclave_server tcp://127.0.0.1:{base_port} --data {node_path / Path('chain')} -p {chain_abci_port} --tx_query tcp://127.0.0.1:{base_port}'''),
        ('tendermint', f'''tendermint node --home={node_path / Path('tendermint')}'''),
        ('client-rpc', f'''client-rpc --port={client_rpc_port} --chain-id={cfg['chain_id']} --storage-dir={node_path / Path('wallet')} --websocket-url=ws://127.0.0.1:{tendermint_rpc_port}/websocket'''),
    ]

    return {
        'program:%s-%s' % (name, node['name']): {
            'command': cmd,
            'stdout_logfile': f"%(here)s/logs/{name}-%(group_name)s.log",
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


def tasks_ini(node_cfgs, app_hash, root_path, cfg):
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
        prgs = programs(node, app_hash, root_path, cfg)
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
        result = await interact(f'dev-utils genesis generate -g "{fp.name}"')
        return json.loads('{%s}' % result.decode('utf-8'))


async def gen_wallet_addr(mnemonic, type='Staking', count=1):
    prefix = {
        'Staking': '0x',
        'Transfer': 'dcro',
    }[type]
    with tempfile.TemporaryDirectory() as dirname:
        await interact(
            f'client-cli wallet restore --name Default',
            ('123456\n123456\n%s\n%s\n' % (mnemonic, mnemonic)).encode(),
            env=dict(
                os.environ,
                CRYPTO_CLIENT_STORAGE=dirname,
            ),
        )
        addrs = []
        for i in range(count):
            result = (await interact(
                f'client-cli address new --name Default --type {type}',
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
        "chain_id": cfg['chain_id'],
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

    patch = jsonpatch.JsonPatch(cfg['chain_config_patch'])
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


def gen_distribution(cfg):
    dist = {
        node['staking'][0]: str(node['bonded_coin'])
        for node in cfg['nodes']
    }

    # burn extra coins
    max_coin = 10000000000000000000
    total_dist = sum(node['bonded_coin'] + node['unbonded_coin'] for node in cfg['nodes'])
    burned = max_coin - total_dist - cfg['expansion_cap']
    if burned > 0:
        dist['0x0000000000000000000000000000000000000000'] = str(burned)

    for node in cfg['nodes']:
        dist[node['staking'][1]] = str(node['unbonded_coin'])
    return dist


def gen_peers(cfgs):
    return ','.join(
        'tcp://%s@%s:%d' % (
            SigningKey(cfg['node_seed']).validator_address().lower(),
            cfg['hostname'],
            cfg['base_port'] + 6
        )
        for i, cfg in enumerate(cfgs)
    )

def write_json(cfg, app_hash, root_path):
    node_cfg= cfg['nodes'][0]
    info= {"app_hash":app_hash, "seed_id":  SigningKey(node_cfg['node_seed']).validator_address().lower()}
    # write
    json.dump(info,
                open(root_path / Path('info.json'), 'w'),
                indent=4)
    # write nodes
    json.dump(cfg,
                open(root_path / Path('nodes_info.json'), 'w'),
                indent=4)

    # write
    f = open("run_test_env.sh", "w")
    f.write("export APP_HASH={}\n".format(info["app_hash"]))
    f.write("export SEED_ID={}\n".format(info["seed_id"]))
    f.close()

async def init_cluster(cfg):
    await populate_wallet_addresses(cfg['nodes'])

    peers = gen_peers(cfg['nodes'])
    genesis = await gen_genesis(cfg)
    app_hash = genesis['app_hash']
    root_path = Path(cfg['root_path']).resolve()

    write_json(cfg, app_hash, root_path)

    for i, node in enumerate(cfg['nodes']):
        base_port = node['base_port']
        node_name = 'node%d' % i
        cfg_path = root_path / Path(node_name) / Path('tendermint') / Path('config')
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

        patch = jsonpatch.JsonPatch(cfg['tendermint_config_patch'])
        toml.dump(
            patch.apply(
                tendermint_cfg(
                    node_name,
                    node['hostname'],
                    26658,
                    26657,
                    26656,
                    ""
                )
            ),
            open(cfg_path / Path('config.toml'), 'w')
        )

        data_path = root_path / Path(node_name) / Path('tendermint') / Path('data')
        if not data_path.exists():
            data_path.mkdir()
        json.dump({
            "height": "0",
            "round": "0",
            "step": 0
        }, open(data_path / Path('priv_validator_state.json'), 'w'))

    logs_path = root_path / Path('logs')
    if not logs_path.exists():
        logs_path.mkdir()
    write_tasks_ini(open(root_path / Path('tasks.ini'), 'w'),
                    tasks_ini(cfg['nodes'], app_hash, root_path, cfg))


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
    def gen(self, count=1, expansion_cap=50000000000000000,
            dist=1000000000000000000,
            genesis_time="2019-11-20T08:56:48.618137Z",
            base_fee='0.0', per_byte_fee='0.0',
            base_port=26650, sgx_device=None,
            chain_id='test-ab', root_path='./', hostname='127.0.0.1'):
        '''Generate testnet node specification
        :param count: Number of nodes, [default: 1].
        '''
        share = int(dist / count / 2)
        sgx_mode = '' if sgx_device else '-sw'
        cfg = {
            'root_path': root_path,
            'chain_id': chain_id,
            'sgx_device': sgx_device,
            'enclave_docker_image': 'integration-tests-chain-tx-enclave' + sgx_mode,
            'genesis_time': genesis_time,
            'expansion_cap': expansion_cap,
            'nodes': [
                {
                    'name': 'node%d' % i,
                    'hostname': hostname.format(index=i),
                    'mnemonic': gen_mnemonic(),
                    'validator_seed': gen_seed(),
                    'node_seed': gen_seed(),
                    'bonded_coin': share,
                    'unbonded_coin': share,
                    'base_port': base_port + (i * 10),
                }
                for i in range(count)
            ],
            'chain_config_patch': [
                {'op': 'replace', 'path': '/initial_fee_policy/base_fee', 'value': '0.0'},
                {'op': 'replace', 'path': '/initial_fee_policy/per_byte_fee', 'value': '0.0'},
            ],
            'tendermint_config_patch': [
                {'op': 'replace', 'path': '/consensus/create_empty_blocks', 'value': True},
                {'op': 'add', 'path': '/consensus/create_empty_blocks_interval', 'value': '0s'},
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
        print('Prepared succesfully', cfg['root_path'])

    def prepare_cfg(self, cfg, spec=None):
        asyncio.run(init_cluster(cfg))
        print('Prepared succesfully', cfg['root_path'])

if __name__ == '__main__':
    fire.Fire(CLI())
