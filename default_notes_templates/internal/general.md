# Scope

(past in scope from workbook excel sheet

---

# PPC


Introductions 
    Let them know that their primary contact will be the PM and there should be 

Go over general attack strategy/procedure. 
    We will get a beacon payload by the time the test starts 
        The beacon payload should be executed on a domain joined windows system. 
            If the system is not domain joined/no domain - let Seth know as this modifies the standard beacon 
        Select a user based on a department/role that they would like tested (Marketing, Sales, HR, IT) 
            This can be a test system with a cloned user, but then we don't get keylogging or screen grabs 
        The beacon is created using Cobalt Strike and communicates over HTTPS 
        Since Cobalt Strike is very well signatured, remind them that they may need to add an exclusion in antivirus and/or web filter 
    We will look at local privilege escalation, conduct portscans, password sprays, targeted vulnerability scanning (NOT NESSUS), lateral movement opportunities, and escalating to DOMAIN ADMIN privilege.  
    Ask if they want a focus on any particular assets. for example, an old time logging system, or remote access system. 

Confirm On Prem AD vs NoAD or Azure AD

- [ ] on prem
- [ ] azure ad
- [ ] hybrid (no on prem dcs)
- [ ] hybrid (on prem dcs)

ask about sensitive systems that scanning may crash
- (system 1)
- (continue as needed)

ask about secondary objective
- (object 1)
- (continue as needed)

ask about emergency contacts

| name | method | contact info |
| ---- | ------ | ------------ |


Ask if they have any questions or concerns 
- question to follow up on 1 
- (continue as needed)

Email any follow-up items from the call to the PM 

