//
//  ViewController.swift
//  IosSample
//
//  Created by leejw51 on 9/3/2020.
//  Copyright © 2020 leejw51. All rights reserved.
//

import UIKit

class ViewController: UIViewController {

    @IBOutlet weak var tendermint_url: UITextField!
    @IBOutlet weak var wallet_name: UITextField!
    @IBOutlet weak var wallet_passphrase: UITextField!
    @IBOutlet weak var wallet_enckey: UITextView!
    
    @IBOutlet weak var wallet_mnemonics: UITextView!
    override func viewDidLoad() {
        super.viewDidLoad()
        // Do any additional setup after loading the view.
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
        let storage = getDocumentsDirectory().appendingPathComponent("disk").path
        print("storage \(storage)")
        print("click wallet = \(name)  passphrase=\(passphrase) mnemonics=\(mnemonics	)")
        restore_wallet(tendermint_url.text, storage, name, passphrase, enckey, mnemonics)
   
        /*let str = "Super long string here"
        let filename = getDocumentsDirectory().appendingPathComponent("output.txt")
        print("filename=\(filename)")

        do {
            try str.write(to: filename, atomically: true, encoding: String.Encoding.utf8)
        } catch {
            // failed to write file – bad permissions, bad filename, missing permissions, or more likely it can't be converted to the encoding
            print("fail to write")
        }
        
        do {
            let text2 = try String(contentsOf: filename, encoding: .utf8)
            print("read = \(text2)")
        }
        catch {/* error handling here */}*/
    }
    
    @IBAction func click_create_sync(_ sender: Any) {
        print("click sync")
    }
}

