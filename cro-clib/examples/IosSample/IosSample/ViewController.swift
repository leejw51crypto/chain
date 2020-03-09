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

    @IBAction func click_create_wallet(_ sender: Any) {
        var name = wallet_name.text!
        var passphrase = wallet_passphrase.text!
        var mnemonics = wallet_mnemonics.text!;
        print("click wallet = \(name)  passphrase=\(passphrase) mnemonics=\(mnemonics	)")
    }
    
    @IBAction func click_create_sync(_ sender: Any) {
        print("click sync")
    }
}

