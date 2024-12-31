deployment to oracle cloud kubernetes is working 

set up local html page to create connection between friends

allow friends to push to your bluetooth device

---

I got 

```PS C:\Users\ryankhetlyr> kubectl cluster-info
WARNING: Permissions on C:\Users\ryankhetlyr\.oci\config are too open.
The following users  / groups have permissions to the file and should not: It's a lion!.
To fix this please try executing the following command:
oci setup repair-file-permissions --file C:\Users\ryankhetlyr\.oci\config
Alternatively to hide this warning, you may set an environment variable; Windows and PowerShell commands follow:
SET OCI_CLI_SUPPRESS_FILE_PERMISSIONS_WARNING=True
$Env:OCI_CLI_SUPPRESS_FILE_PERMISSIONS_WARNING="True"

WARNING: Permissions on C:\Users\ryankhetlyr\.oci\oci_api_key.pem are too open.
The following users  / groups have permissions to the file and should not: It's a lion!.
To fix this please try executing the following command:
oci setup repair-file-permissions --file C:\Users\ryankhetlyr\.oci\oci_api_key.pem
Alternatively to hide this warning, you may set an environment variable; Windows and PowerShell commands follow:
SET OCI_CLI_SUPPRESS_FILE_PERMISSIONS_WARNING=True
$Env:OCI_CLI_SUPPRESS_FILE_PERMISSIONS_WARNING="True"

Kubernetes control plane is running at https://146.235.212.7:6443
CoreDNS is running at https://146.235.212.7:6443/api/v1/namespaces/kube-system/services/kube-dns:dns/proxy
```

---



The manifest validation tests are passing. Now we need to:

1. Configure connectivity to your Oracle Cloud cluster
2. Push our Docker image to Oracle Cloud Container Registry
3. Deploy our application

Obtain

1. The API endpoint (146.235.212.7:6443 from your earlier message)
2. The cluster credentials/kubeconfig file

If you have these, we can:

1. Back up your existing kubeconfig (if any):
```bash
cp ~/.kube/config ~/.kube/config.backup.$(date +%Y%m%d)
```

2. Configure the new kubeconfig. Do you have an OCI CLI config file or a kubeconfig file from the Oracle Cloud console that we can use?

Let me know which credentials you have available and I'll help you set up the connection to your cluster.

---
We've successfully initiated the setup of your Kubernetes infrastructure on Oracle Cloud Free Tier. Here's what we've accomplished:

1. VCN (Virtual Cloud Network) Creation:
- Oracle automatically created a VCN named `oke-vcn-quick-khetlyr-pool-78805b5ba`
- Set up with 10.0.0.0/16 CIDR block
- Includes necessary subnets for Kubernetes

2. Kubernetes Cluster Setup:
- Created a Basic cluster
- Using Kubernetes version v1.31.1
- Set up with Oracle-managed secrets encryption
- Configured for free tier usage
- Credentials valid until 12/27/2029

3. Node Pool Configuration:
- Named `khetlyr-pool`
- Using ARM-based VM.Standard.A1.Flex instances
- 1 OCPU and 6GB memory per node
- 2 managed nodes total

4. Network Access Setup:
- Public API endpoint: 146.235.212.7:6443
- Private API endpoint: 10.0.0.3:6443
- Services CIDR: 10.96.0.0/16
- Load balancer subnets configured

We're ready to deploy the friends-connect application and proceed with preparing for deployment