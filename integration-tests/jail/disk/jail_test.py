#!/usr/bin/python3
import docker
import json
import requests
import datetime
import time

def show_containers() :
	client = docker.from_env()
	containers= client.containers.list()
	for container in containers:
	    id = container
	    print(id.id, id.name)

#show_containers()
# tendermint rpc

server="http://localhost:26657"

def check_validators() :
	try: 
		x= requests.get("http://localhost:26657/validators")
		data =len(x.json()["result"]["validators"])
		print(data)
		return data
	except requests.ConnectionError:
 		return 0
	except:
		assert False

def wait_for_ready() :
	while True:
		validators=check_validators()
		if 2<= validators :
			print("validators ready")
			break
		print("validators =", validators)
		time.sleep(1)


############################################################################3
wait_for_ready()
show_containers()
