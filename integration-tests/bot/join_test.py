#!/usr/bin/python3
import docker
import json
import requests
import datetime
import time
import jsonrpcclient
from chainrpc import RPC, Blockchain
from decouple import config
CURRENT_HASH = config('CURRENT_HASH', '')
class Program :
    def __init__(self) :
        self.rpc = RPC()
        self.blockchain = Blockchain()
        # wallet a
        self.node0_address = ""
        self.node0_address1 = ""
        self.node0_transfer_address = ""
        self.node0_mnemonics= ""

        # wallet b
        self.node1_address = ""
        self.node1_address1= ""
        self.node1_transfer_address = ""
        self.node1_mnemonics=""

        # wallet b
        self.node2_address = ""
        self.node2_mnemonics=""



        self.headers = {
            'Content-Type': 'application/json',
        }

    def get_containers(self) :
        client = docker.from_env()
        containers= client.containers.list()
        ret= {}
        for container in containers:
            id = container
            #ret[id.name]= id.id
            ret[id.name]= container
        return ret
        

    #show_containers()
    # tendermint rpc



    def get_staking_state(self,name, passphrase, addr):
        return self.rpc.staking.state(addr, name)
       

    def create_staking_address(self,name, passphrase):
        return self.rpc.address.create(name,'staking')

    def activate_sync(self):
        print("activate sync")
        self.wallet.sync_unlock("a")
        self.wallet.sync_unlock("b")
        self.wallet.sync_unlock("c")
       
    def restore_wallets(self):
        print("restore wallets")
        self.rpc.wallet.restore(self.node0_mnemonics, "a")
        self.rpc.wallet.restore(self.node1_mnemonics, "b")
        self.rpc.wallet.restore(self.node2_mnemonics, "c")
            

    def create_addresses(self):
        self.create_staking_address("a", "1")
        self.create_staking_address("a", "1")
        self.create_staking_address("b", "1")
        self.create_staking_address("b", "1")
        self.create_staking_address("c", "1")
        self.create_staking_address("c", "1")
        

    def unjail(self,name, passphrase, address):
        try:
            return self.rpc.staking.unjail(address, name)
        except jsonrpcclient.exceptions.JsonRpcClientError as ex:
            print("unjail fail={}".format(ex))

    def check_validators(self) :
        try: 
            x= self.rpc.chain.validators() 
            print(x)
            data =len(x["validators"])
            return data
        except requests.ConnectionError:
            return 0

    def check_validators_old(self) :
        x=self.blockchain.validators()["validators"]
        print("check validators")
        data =len(x)
        print("count={}  check_validators={}".format(data,x))
        return data
      

    def wait_for_ready(self,count) :
        initial_time=time.time() # in seconds
        MAX_TIME = 3600
        while True:
            current_time= time.time()
            elasped_time= current_time - initial_time
            remain_time = MAX_TIME - elasped_time
            validators=self.check_validators()
            if remain_time< 0 :
                assert False
            print("{0}  remain time={1:.2f}  current validators={2}  waiting for validators={3}".format(datetime.datetime.now(), remain_time, validators, count))
            if count== validators :
                print("validators ready")
                break
            time.sleep(10)


    def test_jailing(self) :
        print("test jailing")
        self.wait_for_ready(2)
        containers=self.get_containers()
        print(containers)
        assert "{}_chain1_1".format(CURRENT_HASH) in containers 
        print("wait for jailing")
        time.sleep(10)
        jailthis = containers["{}_chain1_1".format(CURRENT_HASH)]
        print("jail = " , jailthis)
        jailthis.kill()
        self.wait_for_ready(1)
        #jailed
        containers=self.get_containers()
        print(containers)
        assert "{}_chain1_1".format(CURRENT_HASH) not in containers
        print("jail test success")


    def test_unjailing(self) :
        initial_time=time.time() # in seconds
        print("test unjailing")
        self.wait_for_ready(1)

        MAX_TIME = 3600  
        while True:
            current_time= time.time()
            elasped_time= current_time - initial_time
            remain_time = MAX_TIME - elasped_time
            self.check_validators()
            if remain_time< 0 :
                assert False
            self.unjail("b","1", self.node1_address)
            state= self.get_staking_state("b","1", self.node1_address)
            print("state {}".format(state))
            punishment=state["punishment"] 
            print("{0}  remain time={1:.2f}  punishment {2}".format(datetime.datetime.now(), remain_time, punishment))
            if punishment is None :
                print("unjailed!!")
                break
            else :
                print("still jailed")
            time.sleep(10)
        print("unjail test success")

    ############################################################################3
    def main2 (self) :
        self.test_jailing()
        try :
            self.restore_wallets()
            self.activate_sync()
        except jsonrpcclient.exceptions.JsonRpcClientError as ex:
            print("wallet already exists={}".format(ex))
        self.create_addresses()
        self.test_unjailing()

    def prepare(self) :
        try :
            self.restore_wallets()
        except jsonrpcclient.exceptions.JsonRpcClientError as ex:
            print("wallet already exists={}".format(ex))
        self.create_addresses()
        self.rpc.staking.withdraw_all_unbonded(self.node0_address1, self.node0_transfer_address,[], "a")
        self.rpc.wallet.sync_unlock("a")
        self.rpc.wallet.sync("a")
 
    def main (self) :
        #self.prepare()
        time.sleep(2)
        transactions= self.rpc.wallet.transactions("a", 0,1, False)
        assert len(transactions)==1
        tx= transactions[0]
        txid= tx["transaction_id"]
        print("txid={}".format(txid))
        print(transactions)
        self.rpc.staking.deposit(self.node1_address1, [{'id':txid, 'index':0}], "a")
        print("done")


    def read_info(self):
        print("read data")
        with open('info.json') as json_file:
            data = json.load(json_file)
        print(json.dumps(data,indent=4))
        self.node0_address= data["nodes"][0]["staking"][0]
        self.node0_address1= data["nodes"][0]["staking"][1]
        self.node0_transfer_address= data["nodes"][0]["transfer"][0]

        self.node1_address= data["nodes"][1]["staking"][0]
        self.node1_address1= data["nodes"][1]["staking"][1]
        self.node1_transfer_address= data["nodes"][1]["transfer"][0]

        self.node2_address= data["nodes"][2]["staking"][0]

        self.node0_mnemonics=data["nodes"][0]["mnemonic"]
        self.node1_mnemonics=data["nodes"][1]["mnemonic"]
        self.node2_mnemonics=data["nodes"][2]["mnemonic"]
        
    def display_info(self):
        print("jail test current hash={}".format(CURRENT_HASH))
        print("node0 staking= {}".format(self.node0_address))
        print("node0 staking1= {}".format(self.node0_address1))
        print("node0 transfer= {}".format(self.node0_transfer_address))
        print("node1 staking= {}".format(self.node1_address))
        print("node2 staking= {}".format(self.node2_address))
        print("node0 mnemonics= {}".format(self.node0_mnemonics))
        print("node1 mnemonics= {}".format(self.node1_mnemonics))
        print("node2 mnemonics= {}".format(self.node2_mnemonics))


p = Program()
p.read_info()
p.display_info()
p.main()
