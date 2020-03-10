//
//  ViewController.swift
//  IosSample
//
//  Created by leejw51 on 9/3/2020.
//  Copyright © 2020 leejw51. All rights reserved.
//

import UIKit

class ViewController: UIViewController {

    @IBOutlet weak var wallet_name: UITextField!
    @IBOutlet weak var wallet_passphrase: UITextField!
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
        let doc = getDocumentsDirectory()
        let storage = String(format:"%@.storage",doc.absoluteString)
        print("document \(doc)")
        print("storage \(storage)")
        print("click wallet = \(name)  passphrase=\(passphrase) mnemonics=\(mnemonics	)")
        restore_wallet(name, passphrase, mnemonics)
    }
    
    @IBAction func click_create_sync(_ sender: Any) {
        print("click sync")
    }
}

