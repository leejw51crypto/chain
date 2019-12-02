#!/usr/bin/python3
import docker
import json
import requests
import datetime
import time
from decouple import config
from chainrpc import RPC, Blockchain
class Program :
    def __init__(self) :
        self.rpc = RPC()
        self.blockchain = Blockchain()
        self.server='http://127.0.0.1:{}'.format(config('JAIL_CHAIN_RPC'))
        # wallet a
        self.node0_address = ""
        self.node0_mnemonics= ""

        # wallet b
        self.node1_address = ""
        self.node1_mnemonics=""
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
       
    def restore_wallets(self):
        print("restore wallets")
        self.rpc.wallet.restore(self.node0_mnemonics, "a")
        self.rpc.wallet.restore(self.node1_mnemonics, "b")
            

    def create_addresses(self):
        self.create_staking_address("a", "1")
        self.create_staking_address("a", "1")
        self.create_staking_address("b", "1")
        self.create_staking_address("b", "1")
        

    def unjail(self,name, passphrase, address):
        try:
            return self.rpc.staking.unjail(address, name)
        except :
            print("unjail fail")

    def check_validators(self) :
        try: 
            x= requests.get('{}/validators'.format(self.server))
            data =len(x.json()["result"]["validators"])
            return data
        except requests.ConnectionError:
            return 0
        except:
            assert False

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
        if "jail_chain1_1" in containers :
            assert True
        else :
            assert False
        print("wait for jailing")
        time.sleep(10)
        jailthis = containers["jail_chain1_1"]
        print("jail = " , jailthis)
        jailthis.kill()
        self.wait_for_ready(1)
        #jailed
        containers=self.get_containers()
        print(containers)
        if "jail_chain1_1" in containers :
            assert False
        else :
            assert True 
        print("jail test success")


    def test_unjailing(self) :
        initial_time=time.time() # in seconds
        print("test unjailing")
        self.wait_for_ready(1)

        count=2
        MAX_TIME = 3600  
        while True:
            current_time= time.time()
            elasped_time= current_time - initial_time
            remain_time = MAX_TIME - elasped_time
            validators=self.check_validators()
            if remain_time< 0 :
                assert False
            self.unjail("b","1", self.node1_address)
            state= self.get_staking_state("b","1", self.node1_address)
            print("state {}".format(state))
            punishment=state["punishment"] 
            print("{0}  remain time={1:.2f}  punishment {2}".format(datetime.datetime.now(), remain_time, punishment))
            if punishment== None :
                print("unjailed!!")
                break
            else :
                print("still jailed")
            time.sleep(10)
        print("unjail test success")

    ############################################################################3
    def main (self) :
        self.test_jailing()
        try :
            self.restore_wallets()
        except:
            print("wallet already exists")
        self.create_addresses()
        self.test_unjailing()


    def read_info(self):
        print("read data")
        with open('nodes_info.json') as json_file:
            data = json.load(json_file)
        print(json.dumps(data,indent=4))
        self.node0_address= data["nodes"][0]["staking"][0]
        self.node1_address= data["nodes"][1]["staking"][0]

        self.node0_mnemonics=data["nodes"][0]["mnemonic"]
        self.node1_mnemonics=data["nodes"][1]["mnemonic"]
        
    def display_info(self):
        print("node0 staking= {}".format(self.node0_address))
        print("node1 staking= {}".format(self.node1_address))
        print("node0 mnemonics= {}".format(self.node0_mnemonics))
        print("node1 mnemonics= {}".format(self.node1_mnemonics))


p = Program()
p.read_info()
p.display_info()
p.main()
