#!/usr/bin/python3
import docker
import json
import requests
import datetime
import time

def get_containers() :
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

server="http://localhost:26657"

def check_validators() :
	try: 
		x= requests.get('{}/validators'.format(server))
		data =len(x.json()["result"]["validators"])
		return data
	except requests.ConnectionError:
 		return 0
	except:
		assert False

def wait_for_ready(count) :
	while True:
		validators=check_validators()
		print("{}  current validators={}  waiting for validators={}".format(datetime.datetime.now(),validators, count))
		if count== validators :
			print("validators ready")
			break
		time.sleep(1)


def test_jailing() :
    wait_for_ready(2)
    containers=get_containers()
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
    wait_for_ready(1)
    #jailed
    containers=get_containers()
    print(containers)
    if "jail_chain1_1" in containers :
        assert False
    else :
        assert True 
    print("jail test success")


############################################################################3
test_jailing()
