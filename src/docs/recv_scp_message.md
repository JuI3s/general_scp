The diagram below describes how SCP messages are received and interfaces for the various levels of abstraction in the original SCP implementation. Note the name of methods do not reflect exact name of methods in the implementation. 

                    	 ┌──────────────────────────────┐       
                         │       recvSCPEnvelope        │       
┌──────────────┐         │                              │       
│  Nomination  │         │Process the message according │       
│  protocol,   │         │to steps described in the SCP │       
│    Ballot    │────────▶│   white paper, e.g. update   │       
│   protocol   │         │    according to rules of     │       
└──────────────┘         │    federated voting, etc.    │       
        ▲                │                              │       
        │                └──────────────────────────────┘       
        │                                                       
        │                                                       
        │                 ┌────────────────────────────────────┐
        │                 │          recvSCPEnvelope           │
┌──────────────┐          │                                    │
│     Slot     │─────────▶│ Decide whether the envelope is for │
└──────────────┘          │    the nomination or the ballot    │
        ▲                 │protocol and forward it accordingly.│
        │                 └────────────────────────────────────┘
        │                                                       
        │                ┌──────────────────────────────┐       
        │                │       recvSCPEnvelope        │       
┌──────────────┐         │                              │       
│     SCP      │────────▶│Pass the message to a specific│       
└──────────────┘         │       slot to process        │       
        ▲                │                              │       
        │                └──────────────────────────────┘       
        │                                                       
        │                                                       
        │                                                       
        │               ┌───────────────────────────────┐       
        │               │        recvSCPEnvelope        │       
        │               │                               │       
        │               │  Application-level specific   │       
        │               │  verification, e.g. checking  │       
        │               │ digital signatures and if the │       
┌──────────────┐        │ messages should be discarded, │       
│    Herder    │───────▶│   which is the case if the    │       
└──────────────┘        │    messages for an already    │       
        ▲               │  externalized slot or if the  │       
        │               │application is currently out of│       
        │               │ sync. Update various tracking │       
        │               │            metrics            │       
        │               └───────────────────────────────┘       
        │                ┌─────────────────────────────┐        
        │                │recvMessage. Depending on the│        
        │                │  type of the message, call  │        
        │                │ different handlers. Pass to │        
┌──────────────┐         │ Herder::recvSCPmessage for  │        
│     Peer     │────────▶│SCP messages. Update Overlay │        
└──────────────┘         │  metrics, e.g. what remote  │        
        ▲                │ peers have sent us what and │        
        │                │           so on.            │        
        │                └─────────────────────────────┘        
        │                   ┌────────────────────┐              
        │                   │      Receive       │              
┌──────────────┐            │ raw/authenticated  │              
│   Loopback   │            │   message, etc.    │              
│peer/TCP peer │───────────▶│Receive messages at │              
└──────────────┘            │  the bytes level   │              
                            │                    │              
                            │                    │              
                            └────────────────────┘