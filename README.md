# Block-based Komura Equivalence (BKE) 
This is an implementation of BKE as proposed in GPU-based cluster-labeling algorithm without the use of conventional iteration: Application to the Swendsen–Wang multi-cluster spin flip algorithm.
It takes any image up to a size of 6000k x 6000k and creates connected components out of the foreground pixel. Background pixel should be set to black.

This work was heavily inspired by https://github.com/prittt/YACCLAB and [A State-of-the-Art Review with Code about Connected Components Labeling on GPUs](https://ieeexplore.ieee.org/document/10613471/). It implements [Optimized Block-Based Algorithms to Label  Connected Components on GPUs](https://link.springer.com/chapter/10.1007/978-3-030-30645-8_25).
