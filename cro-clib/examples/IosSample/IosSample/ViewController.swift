//
//  ViewController.swift
//  IosSample
//
//  Created by leejw51 on 9/3/2020.
//  Copyright © 2020 leejw51. All rights reserved.
//

import UIKit

class ViewController: UIViewController {
    var my_data: MyData = MyData()
    var filename: URL?
    var my_filename = "info1.json"
    var my_storage = "storage"
    
    @IBOutlet weak var tendermint_url: UITextField!
    @IBOutlet weak var wallet_name: UITextField!
    @IBOutlet weak var wallet_passphrase: UITextField!
    @IBOutlet weak var wallet_enckey: UITextView!
    
    @IBOutlet weak var wallet_mnemonics: UITextView!
    
    func save() throws {
        let jsonEncoder = JSONEncoder()
        let jsonData = try jsonEncoder.encode(my_data)
        let jsonString = String(data: jsonData, encoding: String.Encoding.utf8)!
        try jsonString.write(to: filename!, atomically: true, encoding: String.Encoding.utf8)
        let user = my_data
        print("Save \(user.name!) \(user.mnemonics!)")
    }
    func load() throws {
        let text2 = try String(contentsOf: filename!, encoding: .utf8)
        let jsonData2 = text2.data(using: .utf8)!
        let jsonDecoder = JSONDecoder()
        let user = try jsonDecoder.decode(MyData.self, from: jsonData2)
        my_data=user
        print("Load \(user.name!) \(user.mnemonics!)")
    }
    override func viewDidLoad() {
        super.viewDidLoad()
        do {
            filename=getDocumentsDirectory().appendingPathComponent(my_filename)
            try load()
            tendermint_url.text = my_data.tendermint
            wallet_name.text = my_data.name
            wallet_passphrase.text = my_data.passphras
            wallet_enckey.text = my_data.enckey
            wallet_mnemonics.text = my_data.mnemonics
        }
        catch {
            print("view load error")
        }
    }
    
    func getDocumentsDirectory() -> URL {
        let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
        let documentsDirectory = paths[0]
        return documentsDirectory
    }
    
    @IBAction func click_create_wallet(_ sender: Any) {
        let name = wallet_name.text!
        let  passphrase = wallet_passphrase.text!
        let mnemonics = wallet_mnemonics.text!
        let enckey = wallet_enckey.text!
        let storage = getDocumentsDirectory().appendingPathComponent(my_storage).path
        print("storage \(storage)")
        print("click wallet = \(name)  passphrase=\(passphrase) mnemonics=\(mnemonics	)")
        
        my_data.tendermint = tendermint_url.text
        my_data.name = name
        my_data.passphras = passphrase
        my_data.enckey = enckey
        my_data.mnemonics = mnemonics
        
        do {
            try save()
        }
        catch {
            print("save error")
        }
        
        restore_wallet(tendermint_url.text, storage, name, passphrase, enckey, mnemonics)
    }
    
    @IBAction func click_create_sync(_ sender: Any) {
        print("click sync")
        let name = wallet_name.text!
        let passphrase = wallet_passphrase.text!
        let mnemonics = wallet_mnemonics.text!
        let enckey = wallet_enckey.text!
        let storage = getDocumentsDirectory().appendingPathComponent(my_storage).path
        print("storage \(storage)")
        print("click wallet = \(name)  passphrase=\(passphrase) mnemonics=\(mnemonics    )")

        my_data.tendermint = tendermint_url.text
        my_data.name = name
        my_data.passphras = passphrase
        my_data.enckey = enckey
        my_data.mnemonics = mnemonics

        sync_wallet(tendermint_url.text, storage, name, passphrase, enckey, mnemonics)
    }
    @IBAction func click_default(_ sender: Any) {
        do {
            print("click default")
            tendermint_url.text = "ws://localhost:26657/websocket"
            wallet_name.text = "a"
            wallet_passphrase.text = ""
            wallet_enckey.text = ""
            wallet_mnemonics.text = ""
            try save()
        }
        catch {
            print("click default error")
        }
    }
}

